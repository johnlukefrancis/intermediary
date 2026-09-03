// Path: src-tauri/src/lib/agent/wsl_agent_termination.rs
// Description: The supervisor's WSL emergency stop - TERM, the agent's own drain, then its process trees

//! The emergency bound behind every ordinary WSL stop. The ordinary route is the
//! host agent forwarding `shutdown`, and by the time this runs the agent is
//! normally already gone — the first probe finds nothing and this returns
//! [`WslTerminateOutcome::NoMatch`] without signalling anything, so restart and
//! stop stay as fast as they were.
//!
//! When an agent *is* still there, it is one that has been asked to stop and is
//! draining: its own bound for that is 450 s
//! (`im_agent::server::SHUTDOWN_EMERGENCY_BOUND`), at the end of which it takes
//! the Git process trees it owns with it. So this route waits that drain out
//! rather than killing into it — a `git commit` killed mid-flight leaves
//! `.git/index.lock` behind, and a hook killed with the agent's pid alone is
//! left mutating the worktree with nobody above it (the outer distro termination
//! is deliberately conditional, so it cannot be relied on to sweep up).
//!
//! Only when that envelope expires does this stop being gentle, and then it ends
//! the whole tree rather than the pid: every descendant process group first,
//! then the descendants inside the agent's own group, then the agent
//! ([`super::wsl_process_tree_commands`]).

use super::wsl_agent_discovery::{
    list_exact_wsl_agent_pids, list_reclaimable_wsl_agent_pids_by_port,
    list_wsl_agent_pids_by_port_listener,
};
use super::wsl_agent_termination_channel::LiveChannel;
use super::wsl_process_control::WslLaunchTarget;
use crate::obs::logging;
use std::thread;
use std::time::{Duration, Instant};

/// Every poll is a `wsl.exe` round trip, so the cadence is fast only for the
/// window in which an already-idle agent actually exits, and one second
/// thereafter — a 50 ms cadence held for the whole drain envelope would be
/// thousands of process launches.
const DRAIN_POLL_FAST: Duration = Duration::from_millis(50);
const DRAIN_POLL_FAST_WINDOW: Duration = Duration::from_secs(1);
const DRAIN_POLL_SLOW: Duration = Duration::from_secs(1);

/// The whole envelope one emergency stop is allowed: how long the agent's own
/// drain is waited out after TERM, and the short grace after the tree sweep for
/// the kernel to deliver SIGKILL and `ps` to notice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WslTerminateBudget {
    pub drain_wait: Duration,
    pub kill_grace: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WslTerminateOutcome {
    /// Nothing matched: the ordinary route already finished.
    NoMatch,
    /// The agent exited inside the drain envelope, so its own shutdown owned
    /// everything it had started.
    TerminatedWithTerm,
    /// The envelope expired. `signalled` is how many descendant groups,
    /// same-group descendants, and agents the in-distro sweep actually reached.
    TerminatedWithKill { signalled: usize },
}

/// The three things an emergency stop does inside the distro. One trait so the
/// orchestration above it is exercised without a live `wsl.exe` — and so the
/// production path has exactly one implementation.
pub(super) trait WslTerminationChannel {
    /// Every process this stop is responsible for, right now.
    fn list_pids(&mut self) -> Result<Vec<u32>, String>;
    /// Asks them to drain. `Ok(Some(_))` is a signal command that reported a
    /// failure but left the route usable.
    fn send_term(&mut self, pids: &[u32]) -> Result<Option<String>, String>;
    /// Kills each pid's descendant process groups, then the pids. Answers how
    /// many groups and pids the sweep reached.
    fn kill_process_trees(&mut self, pids: &[u32]) -> Result<usize, String>;
}

pub fn terminate_wsl_agent_process(
    target: &WslLaunchTarget,
    budget: WslTerminateBudget,
) -> Result<WslTerminateOutcome, String> {
    if !cfg!(target_os = "windows") {
        return Ok(WslTerminateOutcome::NoMatch);
    }

    terminate_matching_wsl_agent_processes(
        &mut LiveChannel::new(target.distro.as_deref(), || {
            list_exact_wsl_agent_pids(target)
        }),
        budget,
        &target.agent_bin_wsl,
    )
}

pub fn terminate_intermediary_wsl_agent_processes_by_port(
    target: &WslLaunchTarget,
    wsl_port: u16,
    budget: WslTerminateBudget,
) -> Result<WslTerminateOutcome, String> {
    if !cfg!(target_os = "windows") {
        return Ok(WslTerminateOutcome::NoMatch);
    }

    terminate_matching_wsl_agent_processes(
        &mut LiveChannel::new(target.distro.as_deref(), || {
            list_reclaimable_wsl_agent_pids_by_port(target, wsl_port)
        }),
        budget,
        &format!("INTERMEDIARY_AGENT_PORT={wsl_port} or port-listener :{wsl_port}"),
    )
}

/// Reclaims any Intermediary `im_agent` bound to `wsl_port`, using only the port-listener probe.
/// Used by config-less callers (`stop`, app exit) that know the distro and port but may not hold a
/// full launch target for the running backend.
pub fn terminate_wsl_agent_by_port_listener(
    distro: Option<&str>,
    wsl_port: u16,
    budget: WslTerminateBudget,
) -> Result<WslTerminateOutcome, String> {
    if !cfg!(target_os = "windows") {
        return Ok(WslTerminateOutcome::NoMatch);
    }

    terminate_matching_wsl_agent_processes(
        &mut LiveChannel::new(distro, || {
            list_wsl_agent_pids_by_port_listener(distro, wsl_port)
        }),
        budget,
        &format!("port-listener :{wsl_port}"),
    )
}

pub(super) fn terminate_matching_wsl_agent_processes<C: WslTerminationChannel>(
    channel: &mut C,
    budget: WslTerminateBudget,
    match_description: &str,
) -> Result<WslTerminateOutcome, String> {
    let matching_pids = channel.list_pids()?;
    if matching_pids.is_empty() {
        return Ok(WslTerminateOutcome::NoMatch);
    }

    let mut signal_errors: Vec<String> = Vec::new();
    if let Some(error) = channel.send_term(&matching_pids)? {
        signal_errors.push(error);
    }

    let started = Instant::now();
    let drain = wait_for_wsl_agent_exit(channel, budget.drain_wait);
    if drain.exited {
        log_phase(
            "info",
            "drain_wait",
            "drained_by_agent",
            match_description,
            &format!(
                "elapsedMs={} probeFailures={}",
                started.elapsed().as_millis(),
                drain.probe_failures
            ),
        );
        return Ok(WslTerminateOutcome::TerminatedWithTerm);
    }

    // The envelope is over, so this re-read is authoritative: a probe that
    // cannot answer here fails the stop instead of escalating blind.
    let surviving_pids = channel.list_pids()?;
    if surviving_pids.is_empty() {
        log_phase(
            "info",
            "drain_wait",
            "drained_by_agent",
            match_description,
            &format!(
                "elapsedMs={} probeFailures={} detail=exited_at_envelope_edge",
                started.elapsed().as_millis(),
                drain.probe_failures
            ),
        );
        return Ok(WslTerminateOutcome::TerminatedWithTerm);
    }

    log_phase(
        "warn",
        "drain_wait",
        "expired",
        match_description,
        &format!(
            "budgetMs={} probeFailures={} survivingPids={}",
            budget.drain_wait.as_millis(),
            drain.probe_failures,
            surviving_pids.len()
        ),
    );

    let signalled = channel.kill_process_trees(&surviving_pids)?;
    let grace = wait_for_wsl_agent_exit(channel, budget.kill_grace);
    if grace.exited {
        log_phase(
            "warn",
            "tree_kill",
            "terminated",
            match_description,
            &format!("signalled={signalled} agents={}", surviving_pids.len()),
        );
        return Ok(WslTerminateOutcome::TerminatedWithKill { signalled });
    }

    log_phase(
        "error",
        "tree_kill",
        "survived",
        match_description,
        &format!("signalled={signalled} agents={}", surviving_pids.len()),
    );
    let mut error = format!(
        "WSL agent process matched by {match_description} did not exit after TERM, a {}ms drain wait, and a process-tree KILL that signalled {signalled} groups/pids",
        budget.drain_wait.as_millis()
    );
    if !signal_errors.is_empty() {
        error = format!("{error}. {}", signal_errors.join("; "));
    }
    Err(error)
}

/// What one bounded wait saw. A probe that could not answer is not proof the
/// agent is gone, so it never ends the wait: over a 480 s envelope made of
/// `wsl.exe` round trips a single hiccup must not skip the escalation behind it.
/// The count travels to the log, and the authoritative re-read after the
/// envelope is what propagates a channel that is genuinely broken.
struct ExitWait {
    exited: bool,
    probe_failures: usize,
}

fn wait_for_wsl_agent_exit<C: WslTerminationChannel>(channel: &mut C, bound: Duration) -> ExitWait {
    let start = Instant::now();
    let mut probe_failures = 0usize;
    loop {
        match channel.list_pids() {
            Ok(pids) if pids.is_empty() => {
                return ExitWait {
                    exited: true,
                    probe_failures,
                }
            }
            Ok(_) => {}
            Err(err) => {
                probe_failures += 1;
                if probe_failures == 1 {
                    logging::log(
                        "warn",
                        "agent",
                        "wsl_terminate",
                        &format!("kind=wsl phase=drain_wait outcome=probe_failed error={err}"),
                    );
                }
            }
        }

        let elapsed = start.elapsed();
        if elapsed >= bound {
            return ExitWait {
                exited: false,
                probe_failures,
            };
        }
        let cadence = if elapsed < DRAIN_POLL_FAST_WINDOW {
            DRAIN_POLL_FAST
        } else {
            DRAIN_POLL_SLOW
        };
        thread::sleep(cadence.min(bound - elapsed));
    }
}

fn log_phase(level: &str, phase: &str, outcome: &str, match_description: &str, detail: &str) {
    logging::log(
        level,
        "agent",
        "wsl_terminate",
        &format!("kind=wsl phase={phase} outcome={outcome} match=\"{match_description}\" {detail}"),
    );
}

#[cfg(test)]
#[path = "wsl_agent_termination_tests.rs"]
mod tests;
