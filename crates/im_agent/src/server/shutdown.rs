// Path: crates/im_agent/src/server/shutdown.rs
// Description: The one drain-then-exit owner shared by the shutdown command and the process signals

//! Both agents stop the same way, whether the request arrives as a `shutdown`
//! command on the authenticated socket or as SIGTERM/ctrl-c: admission of new
//! mutations is closed, the mutations already running are given a bounded
//! window to reach a terminal state, and only then does the process exit.
//!
//! Killing a running `git commit` bypasses Git's own lockfile cleanup and
//! leaves `.git/index.lock` behind, so the drain — not the kill — is the
//! ordinary path. `drained: false` is never itself a reason to exit: the
//! agent keeps waiting until idle, up to the emergency bound below. Only at
//! that bound does the process actually leave, and only there does it take the
//! Git process trees it still owns with it, so nothing of this agent's work
//! outlives the agent unowned. The residue is reported and logged rather than
//! hidden.

use std::process;
use std::time::Duration;

use im_bundle::git::terminate_git_process_trees;
use serde_json::json;

use crate::logging::Logger;
use crate::source_control::SourceControlLocks;

/// The real bound on a shutdown drain. Sized above every bounded mutation this
/// agent can still be running when the request arrives — status 100 s +
/// remote 180 s + status 100 s, plus margin — so a truthful `drained: false`
/// is reserved for a mutation that has genuinely wedged, not one still inside
/// its own timeout. A mutation still running here is past every bound it had,
/// so the bound is also where this agent stops being gentle: the Git process
/// trees it still owns are terminated through the runner's own forced-stop
/// path (`im_bundle::git::terminate_git_process_trees`) before the exit, so no
/// hook, `ssh`, or credential helper of ours outlives the process that owns
/// it. The residue's effect stays `unknown` — that is what killing it means.
pub const SHUTDOWN_EMERGENCY_BOUND: Duration = Duration::from_secs(450);

/// The gap between flushing the response and exiting, so the caller reads its
/// `shutdownResult` before the socket dies.
pub const SHUTDOWN_EXIT_DELAY: Duration = Duration::from_secs(1);

/// What one drain achieved: `drained` is the honest answer to "did everything
/// this agent owned finish", `active_mutations` the residue still holding a
/// worktree lock when the budget expired.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrainOutcome {
    pub drained: bool,
    pub active_mutations: u32,
}

impl DrainOutcome {
    pub fn idle() -> Self {
        Self {
            drained: true,
            active_mutations: 0,
        }
    }
}

/// Closes admission and waits for the running mutations, up to the emergency
/// bound. Reads keep being served throughout: only mutations are refused
/// (`AGENT_DRAINING`, proven `notApplied`), so a UI that is still up gets a
/// truthful status. Never returns early with `drained: false` on any budget
/// short of the emergency bound — the caller answers and exits only with what
/// this returns.
pub async fn drain_source_control(
    locks: &SourceControlLocks,
    logger: &Logger,
    reason: &str,
) -> DrainOutcome {
    drain_source_control_bounded(locks, logger, reason, SHUTDOWN_EMERGENCY_BOUND).await
}

/// The same drain, but bounded by a caller-supplied budget rather than the
/// full emergency bound. The host agent uses this: it shares one emergency
/// envelope with the WSL backend it forwards to (deadline in, remaining time
/// out), so the two together never wait longer than a lone agent would.
pub async fn drain_source_control_bounded(
    locks: &SourceControlLocks,
    logger: &Logger,
    reason: &str,
    bound: Duration,
) -> DrainOutcome {
    locks.set_draining();
    let active_at_entry = locks.busy_count();
    let drained = locks.wait_idle(bound).await;
    let active_mutations = locks.busy_count();
    let outcome = DrainOutcome {
        drained,
        active_mutations,
    };

    let details = json!({
        "reason": reason,
        "activeAtEntry": active_at_entry,
        "activeMutations": active_mutations,
        "budgetMs": bound.as_millis(),
        "drained": drained,
    });
    if drained {
        logger.info("Source-control drain complete", Some(details));
    } else {
        // The bound expired with the residue still holding a worktree lock.
        // Nothing about the repository is proven either way: its outcome is
        // unknown, not failed, and `finalize_shutdown` is what makes it final.
        logger.warn(
            "Source-control drain reached its bound; residue outcome is unknown",
            Some(details),
        );
    }
    outcome
}

/// The last act of a shutdown, whichever route asked for it — the `shutdown`
/// command or a signal. A drain that finished has nothing left to own. One that
/// did not means a Git command is running past every bound it had, so the
/// process trees this agent still owns are terminated through the runner's own
/// forced-stop path (`im_bundle::git::terminate_git_process_trees`) rather than
/// orphaned by the exit that follows. Reports how many trees were reached.
///
/// Separate from the drain on purpose: the drain answers a question, this ends
/// things, and only the route that is actually ending the process calls it.
pub async fn finalize_shutdown(logger: &Logger, reason: &str, outcome: DrainOutcome) -> usize {
    if outcome.drained {
        return 0;
    }
    // A handful of syscalls under one registry lock, but it is the runner's own
    // blocking path, so it runs off the async runtime like every other Git
    // interaction (ADR-009).
    let terminated = tokio::task::spawn_blocking(terminate_git_process_trees)
        .await
        .unwrap_or(0);
    logger.warn(
        "Terminated the Git process trees this agent still owned at shutdown",
        Some(json!({
            "reason": reason,
            "activeMutations": outcome.active_mutations,
            "terminatedProcessTrees": terminated,
            "effect": "unknown",
        })),
    );
    terminated
}

/// Exits the process shortly after the caller's response has been handed to the
/// writer task. Detached on purpose: the request that asked for shutdown must
/// be able to answer before the process is gone.
pub fn schedule_process_exit(logger: Logger, reason: &'static str) {
    tokio::spawn(async move {
        // Logged before the wait, not after it: the log writer is another task,
        // and `process::exit` gives it no chance to drain.
        logger.info(
            "Agent process exiting after shutdown",
            Some(json!({
                "reason": reason,
                "exitCode": 0,
                "afterMs": SHUTDOWN_EXIT_DELAY.as_millis(),
            })),
        );
        tokio::time::sleep(SHUTDOWN_EXIT_DELAY).await;
        process::exit(0);
    });
}

/// Resolves when the operating system asks this process to stop. SIGTERM is the
/// signal a supervisor or `wsl --terminate` actually sends; ctrl-c is the
/// interactive equivalent.
pub async fn wait_for_shutdown_signal(logger: &Logger) -> &'static str {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut terminate = match signal(SignalKind::terminate()) {
            Ok(stream) => stream,
            Err(err) => {
                logger.warn(
                    "Failed to install the SIGTERM handler",
                    Some(json!({"error": err.to_string()})),
                );
                return wait_for_ctrl_c(logger).await;
            }
        };
        tokio::select! {
            _ = terminate.recv() => "sigterm",
            reason = wait_for_ctrl_c(logger) => reason,
        }
    }
    #[cfg(not(unix))]
    {
        wait_for_ctrl_c(logger).await
    }
}

async fn wait_for_ctrl_c(logger: &Logger) -> &'static str {
    if let Err(err) = tokio::signal::ctrl_c().await {
        logger.warn(
            "Failed to listen for the ctrl-c signal",
            Some(json!({"error": err.to_string()})),
        );
        // The listener is gone; never spin on an error that returns at once.
        std::future::pending::<()>().await;
    }
    "ctrl_c"
}

#[cfg(test)]
mod tests;
