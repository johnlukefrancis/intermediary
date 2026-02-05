// Path: crates/im_host_agent/src/server/dispatch.rs
// Description: Host-agent command dispatch over routed runtime backends

use im_agent::error::AgentError;
use im_agent::protocol::{UiCommand, UiResponse};

use super::connection::ConnectionContext;

pub async fn dispatch_command(
    command: UiCommand,
    ctx: &ConnectionContext,
) -> Result<UiResponse, AgentError> {
    let mut runtime = ctx.runtime.write().await;
    runtime
        .dispatch_command(command, &ctx.agent_version, &ctx.event_bus)
        .await
}
