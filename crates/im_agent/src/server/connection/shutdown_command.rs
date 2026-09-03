// Path: crates/im_agent/src/server/connection/shutdown_command.rs
// Description: The `shutdown` command handler for the WSL agent: drain, answer, then exit

use crate::error::AgentError;
use crate::protocol::{ShutdownResult, UiResponse};
use crate::server::shutdown::{drain_source_control, finalize_shutdown, schedule_process_exit};

use super::ConnectionContext;

/// Drains this agent and answers with what the drain achieved. The process exit
/// is scheduled, not performed: the response has to leave the socket first, and
/// the writer task owns that. Reads keep working for the whole drain, so a UI
/// that is still up can watch the mutations finish.
pub async fn shutdown_command(ctx: &ConnectionContext) -> Result<UiResponse, AgentError> {
    let locks = {
        let state = ctx.runtime.read().await;
        state.source_control_locks.clone()
    };
    let outcome = drain_source_control(&locks, &ctx.logger, "shutdownCommand").await;
    finalize_shutdown(&ctx.logger, "shutdownCommand", outcome).await;
    schedule_process_exit(ctx.logger.clone(), "shutdownCommand");
    Ok(UiResponse::ShutdownResult(ShutdownResult {
        drained: outcome.drained,
        active_mutations: outcome.active_mutations,
    }))
}
