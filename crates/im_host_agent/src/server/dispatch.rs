// Path: crates/im_host_agent/src/server/dispatch.rs
// Description: Host-agent command dispatch: WSL forwards and long host work run after the runtime lock is released

use im_agent::error::AgentError;
use im_agent::protocol::{SetOptionsCommand, UiCommand, UiResponse};

use super::connection::ConnectionContext;
use super::shutdown_dispatch;
use crate::runtime::{
    execute_host_import, execute_host_source_control, execute_host_worktree_action,
    run_host_build_bundle,
};

/// Locks are taken only to read or apply runtime state; no lock spans a WSL or long host await.
pub async fn dispatch_command(
    command: UiCommand,
    ctx: &ConnectionContext,
    inspection_cancel: Option<im_agent::bundles::BundleCancelToken>,
) -> Result<UiResponse, AgentError> {
    match command {
        // Shutdown drains the WSL backend and then this process; both waits are minutes long.
        UiCommand::Shutdown => return shutdown_dispatch::dispatch_shutdown(ctx).await,
        UiCommand::SetOptions(command) => return dispatch_set_options(command, ctx).await,
        _ => {}
    }

    let wsl_client = {
        let runtime = ctx.runtime.read().await;
        runtime.wsl_client_for_command(&command)?
    };
    if let Some(client) = wsl_client {
        return client.forward_routed(command).await;
    }

    match command {
        UiCommand::BuildBundle(command) => {
            let options = {
                let runtime = ctx.runtime.read().await;
                runtime.prepare_host_build_bundle(command)?
            };
            run_host_build_bundle(options, &ctx.event_bus, &ctx.logger).await
        }
        UiCommand::CancelBundleBuild(command) => {
            let runtime = ctx.runtime.read().await;
            Ok(runtime.dispatch_host_cancel_bundle_build(command))
        }
        UiCommand::InspectBundleEntry(_)
        | UiCommand::SourceControlStatus(_)
        | UiCommand::SourceControlDiff(_)
        | UiCommand::SourceControlImageDiff(_)
        | UiCommand::SourceControlAction(_)
        | UiCommand::ImportFiles(_)
        | UiCommand::WorktreeAction(_) => {
            dispatch_unlocked_host_command(command, ctx, inspection_cancel).await
        }
        command => {
            let mut runtime = ctx.runtime.write().await;
            runtime
                .dispatch_command(command, &ctx.agent_version, &ctx.event_bus)
                .await
        }
    }
}

/// Options apply locally under the lock; the WSL copy is forwarded after release.
async fn dispatch_set_options(
    command: SetOptionsCommand,
    ctx: &ConnectionContext,
) -> Result<UiResponse, AgentError> {
    let (result, wsl_client) = {
        let mut runtime = ctx.runtime.write().await;
        runtime.apply_set_options(command.clone())
    };
    if let Some(client) = wsl_client {
        // A WSL failure is reported as a transport event; the host options already applied.
        let _ = client
            .forward_unrouted(UiCommand::SetOptions(command))
            .await;
    }
    Ok(UiResponse::SetOptionsResult(result))
}

/// Long host repository operations clone their context under a short lock and run unlocked.
async fn dispatch_unlocked_host_command(
    command: UiCommand,
    ctx: &ConnectionContext,
    inspection_cancel: Option<im_agent::bundles::BundleCancelToken>,
) -> Result<UiResponse, AgentError> {
    let repo_id = command
        .repo_id()
        .map(str::to_string)
        .ok_or_else(|| AgentError::new("UNKNOWN_COMMAND", "Unsupported command"))?;
    let context = {
        let runtime = ctx.runtime.read().await;
        runtime.host_source_control_context(&repo_id)?
    };
    match command {
        UiCommand::InspectBundleEntry(command) => {
            let cancel = inspection_cancel
                .ok_or_else(|| AgentError::internal("Inspection cancellation owner missing"))?;
            let result =
                im_agent::bundles::inspect_bundle_entry(context.repo_root, command, cancel).await?;
            Ok(UiResponse::InspectBundleEntryResult(result))
        }
        UiCommand::ImportFiles(command) => execute_host_import(command, context).await,
        UiCommand::WorktreeAction(command) => execute_host_worktree_action(command, context).await,
        command => execute_host_source_control(command, context).await,
    }
}

#[cfg(test)]
#[path = "dispatch_tests.rs"]
mod tests;
