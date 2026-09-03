// Path: crates/im_host_agent/src/server/dispatch.rs
// Description: Host-agent command dispatch over routed runtime backends

use im_agent::error::AgentError;
use im_agent::protocol::{UiCommand, UiResponse};

use super::connection::ConnectionContext;
use super::shutdown_dispatch;
use crate::runtime::{execute_host_source_control, RepoBackend};

pub async fn dispatch_command(
    command: UiCommand,
    ctx: &ConnectionContext,
) -> Result<UiResponse, AgentError> {
    match command {
        UiCommand::BuildBundle(command) => {
            return dispatch_build_bundle(command, ctx).await;
        }
        UiCommand::CancelBundleBuild(command) => {
            return dispatch_cancel_bundle_build(command, ctx).await;
        }
        UiCommand::SourceControlStatus(_)
        | UiCommand::SourceControlDiff(_)
        | UiCommand::SourceControlAction(_) => {
            return dispatch_source_control(command, ctx).await;
        }
        // Shutdown drains the WSL backend and then this process; both waits are
        // minutes long, so it never touches the runtime write lock.
        UiCommand::Shutdown => {
            return shutdown_dispatch::dispatch_shutdown(ctx).await;
        }
        command => {
            let mut runtime = ctx.runtime.write().await;
            runtime
                .dispatch_command(command, &ctx.agent_version, &ctx.event_bus)
                .await
        }
    }
}

async fn dispatch_build_bundle(
    command: im_agent::protocol::BuildBundleCommand,
    ctx: &ConnectionContext,
) -> Result<UiResponse, AgentError> {
    let route_command = UiCommand::BuildBundle(command.clone());
    let backend = {
        let runtime = ctx.runtime.read().await;
        runtime.repo_backend_for_command(&route_command)?
    };

    match backend {
        Some(RepoBackend::Host) => {
            let runtime = ctx.runtime.read().await;
            runtime
                .dispatch_host_build_bundle(command, &ctx.event_bus)
                .await
        }
        Some(RepoBackend::Wsl) => {
            let client = {
                let mut runtime = ctx.runtime.write().await;
                runtime
                    .prepare_wsl_client_for_command(&route_command, &ctx.event_bus)
                    .await?
            };
            let repo_id = command.repo_id.clone();
            let result = client
                .forward_command_with_generation(UiCommand::BuildBundle(command))
                .await;
            let mut runtime = ctx.runtime.write().await;
            match result {
                Ok(forwarded) => {
                    runtime.mark_wsl_forward_success(&ctx.event_bus, forwarded.generation);
                    Ok(forwarded.response)
                }
                Err(err) => {
                    runtime.emit_wsl_forward_error(&err, &ctx.event_bus, Some(repo_id));
                    Err(err)
                }
            }
        }
        None => Err(AgentError::new("UNKNOWN_COMMAND", "Unsupported command")),
    }
}

async fn dispatch_cancel_bundle_build(
    command: im_agent::protocol::CancelBundleBuildCommand,
    ctx: &ConnectionContext,
) -> Result<UiResponse, AgentError> {
    let route_command = UiCommand::CancelBundleBuild(command.clone());
    let backend = {
        let runtime = ctx.runtime.read().await;
        runtime.repo_backend_for_command(&route_command)?
    };

    match backend {
        Some(RepoBackend::Host) => {
            let runtime = ctx.runtime.read().await;
            Ok(runtime.dispatch_host_cancel_bundle_build(command))
        }
        Some(RepoBackend::Wsl) => {
            let client = {
                let mut runtime = ctx.runtime.write().await;
                runtime
                    .prepare_wsl_client_for_command(&route_command, &ctx.event_bus)
                    .await?
            };
            let repo_id = command.repo_id.clone();
            let result = client
                .forward_command_with_generation(UiCommand::CancelBundleBuild(command))
                .await;
            let mut runtime = ctx.runtime.write().await;
            match result {
                Ok(forwarded) => {
                    runtime.mark_wsl_forward_success(&ctx.event_bus, forwarded.generation);
                    Ok(forwarded.response)
                }
                Err(err) => {
                    runtime.emit_wsl_forward_error(&err, &ctx.event_bus, Some(repo_id));
                    Err(err)
                }
            }
        }
        None => Err(AgentError::new("UNKNOWN_COMMAND", "Unsupported command")),
    }
}

/// Source-control commands run Git for up to minutes; they resolve their
/// backend under a short read lock and then run with no runtime lock held, so
/// a long push never freezes other repos, clientHello, or WSL forwards.
async fn dispatch_source_control(
    command: UiCommand,
    ctx: &ConnectionContext,
) -> Result<UiResponse, AgentError> {
    let repo_id = command
        .repo_id()
        .map(str::to_string)
        .ok_or_else(|| AgentError::new("UNKNOWN_COMMAND", "Unsupported command"))?;
    let backend = {
        let runtime = ctx.runtime.read().await;
        runtime.repo_backend_for_command(&command)?
    };

    match backend {
        Some(RepoBackend::Host) => {
            let context = {
                let runtime = ctx.runtime.read().await;
                runtime.host_source_control_context(&repo_id)?
            };
            execute_host_source_control(command, context).await
        }
        Some(RepoBackend::Wsl) => {
            let client = {
                let mut runtime = ctx.runtime.write().await;
                runtime
                    .prepare_wsl_client_for_command(&command, &ctx.event_bus)
                    .await?
            };
            let result = client.forward_command_with_generation(command).await;
            let mut runtime = ctx.runtime.write().await;
            match result {
                Ok(forwarded) => {
                    runtime.mark_wsl_forward_success(&ctx.event_bus, forwarded.generation);
                    Ok(forwarded.response)
                }
                Err(err) => {
                    runtime.emit_wsl_forward_error(&err, &ctx.event_bus, Some(repo_id));
                    Err(err)
                }
            }
        }
        None => Err(AgentError::new("UNKNOWN_COMMAND", "Unsupported command")),
    }
}
