// Path: crates/im_host_agent/src/server/shutdown_dispatch.rs
// Description: Host-agent shutdown: drain the WSL backend first, then this process, then exit

use std::time::{Duration, Instant};

use im_agent::error::AgentError;
use im_agent::logging::Logger;
use im_agent::protocol::{ShutdownResult, UiCommand, UiResponse};
use im_agent::server::{
    drain_source_control_bounded, finalize_shutdown, schedule_process_exit, DrainOutcome,
    SHUTDOWN_EMERGENCY_BOUND,
};
use serde_json::json;
use tokio::time::sleep;

use crate::error_codes::WSL_BACKEND_UNAVAILABLE;
use crate::runtime::HostShutdownTargets;
use crate::wsl::WslBackendClient;

use super::connection::ConnectionContext;

/// Between retries while the backend is unavailable but a mutation is still
/// outstanding: matches the client's own reconnect cadence, so a retry lands
/// right after a fresh connection had a chance to form.
const WSL_UNAVAILABLE_RETRY_INTERVAL: Duration = Duration::from_millis(750);

/// The host agent owns the WSL agent's lifetime, so it drains outward-in: the
/// WSL backend is asked to finish its mutations first (its Git children are the
/// ones that leave `.git/index.lock` behind), then this process drains its own
/// host-root mutations, and only then is the exit scheduled.
///
/// The reported result is the union: `drained` only if both sides finished, and
/// `activeMutations` the sum of what each side still had running.
pub async fn dispatch_shutdown(ctx: &ConnectionContext) -> Result<UiResponse, AgentError> {
    let targets = {
        let runtime = ctx.runtime.read().await;
        runtime.shutdown_targets()
    };
    let logger = targets.logger.clone();

    let outcome = drain_for_shutdown(targets, "shutdownCommand").await;
    finalize_shutdown(&logger, "shutdownCommand", outcome).await;
    schedule_process_exit(logger, "shutdownCommand");

    Ok(UiResponse::ShutdownResult(ShutdownResult {
        drained: outcome.drained,
        active_mutations: outcome.active_mutations,
    }))
}

/// The whole drain, shared by the `shutdown` command and the process signals so
/// SIGTERM cannot become a second, weaker route. Both phases share ONE
/// emergency envelope (`SHUTDOWN_EMERGENCY_BOUND`) rather than each getting
/// their own: the WSL round trip spends from it first, and whatever remains
/// bounds this process's own drain, so the host never keeps a caller waiting
/// longer than a lone agent's own emergency bound would.
pub(super) async fn drain_for_shutdown(targets: HostShutdownTargets, reason: &str) -> DrainOutcome {
    // Host admission closes before the WSL round trip, not after it: a new
    // mutation started while we wait on WSL would have to be drained too.
    targets.locks.set_draining();

    let deadline = Instant::now() + SHUTDOWN_EMERGENCY_BOUND;
    let wsl = drain_wsl_backend(targets.wsl_client, &targets.logger, deadline).await;
    let host_bound = deadline.saturating_duration_since(Instant::now());
    let host =
        drain_source_control_bounded(&targets.locks, &targets.logger, reason, host_bound).await;

    DrainOutcome {
        drained: wsl.drained && host.drained,
        active_mutations: wsl.active_mutations.saturating_add(host.active_mutations),
    }
}

/// Forwards `shutdown` to the WSL agent over the existing authenticated
/// connection, spending from the shared `deadline`. An offline backend only
/// counts as drained when nothing of ours was left outstanding there
/// (`WslBackendClient::has_outstanding_mutations`); otherwise this keeps
/// retrying — a fresh connection answers with the WSL agent's own
/// authoritative lock state — until `deadline` passes.
async fn drain_wsl_backend(
    client: Option<WslBackendClient>,
    logger: &Logger,
    deadline: Instant,
) -> DrainOutcome {
    let Some(client) = client else {
        logger.info(
            "Shutdown skipped the WSL backend: no client was ever connected",
            Some(json!({"drained": true})),
        );
        return DrainOutcome::idle();
    };

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            let residue = client.outstanding_mutation_count();
            logger.warn(
                "WSL backend shutdown wait reached the emergency bound with a mutation still outstanding",
                Some(json!({"activeMutations": residue})),
            );
            return DrainOutcome {
                drained: false,
                active_mutations: residue,
            };
        }

        match client
            .forward_command_with_timeout(UiCommand::Shutdown, remaining)
            .await
        {
            Ok(forwarded) => match forwarded.response {
                UiResponse::ShutdownResult(result) => {
                    logger.info(
                        "WSL backend drained for shutdown",
                        Some(json!({
                            "drained": result.drained,
                            "activeMutations": result.active_mutations,
                            "generation": forwarded.generation,
                        })),
                    );
                    return DrainOutcome {
                        drained: result.drained,
                        active_mutations: result.active_mutations,
                    };
                }
                other => {
                    logger.warn(
                        "WSL backend answered shutdown with an unexpected response",
                        Some(json!({"response": response_name(&other)})),
                    );
                    return undrained();
                }
            },
            Err(err) if err.code() == WSL_BACKEND_UNAVAILABLE => {
                if !client.has_outstanding_mutations() {
                    logger.info(
                        "WSL backend is offline at shutdown; nothing of ours is running there",
                        Some(json!({"code": err.code(), "drained": true})),
                    );
                    return DrainOutcome::idle();
                }
                logger.info(
                    "WSL backend is offline at shutdown with a mutation still outstanding; waiting",
                    Some(json!({
                        "code": err.code(),
                        "activeMutations": client.outstanding_mutation_count(),
                    })),
                );
                sleep(WSL_UNAVAILABLE_RETRY_INTERVAL.min(remaining)).await;
            }
            Err(err) => {
                logger.warn(
                    "WSL backend did not confirm its drain",
                    Some(json!({"code": err.code(), "error": err.message()})),
                );
                return undrained();
            }
        }
    }
}

/// The WSL side neither confirmed nor denied: `drained` is false, and the count
/// stays 0 because nobody counted it. Only a real `shutdownResult` carries a
/// residue number.
fn undrained() -> DrainOutcome {
    DrainOutcome {
        drained: false,
        active_mutations: 0,
    }
}

fn response_name(response: &UiResponse) -> String {
    serde_json::to_value(response)
        .ok()
        .and_then(|value| {
            value
                .get("type")
                .and_then(|kind| kind.as_str().map(str::to_string))
        })
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests;
