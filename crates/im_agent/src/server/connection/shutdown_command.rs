// Path: crates/im_agent/src/server/connection/shutdown_command.rs
// Description: The `shutdown` command handler for the WSL agent: drain, answer, then exit

use crate::error::AgentError;
use crate::protocol::{ShutdownResult, UiResponse};
use crate::server::shutdown::{drain_source_control, finalize_shutdown, schedule_process_exit};

use super::ConnectionContext;

/// Releases the port, drains, answers, then schedules the exit after the writer flushes;
/// connections already open keep their reads for the whole drain.
pub async fn shutdown_command(ctx: &ConnectionContext) -> Result<UiResponse, AgentError> {
    ctx.listener_release.request();
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
