// Path: crates/im_agent/src/server/connection/source_control_commands.rs
// Description: Source-control command handlers for WebSocket dispatch (status, diff, actions)

use std::path::Path;

use crate::error::AgentError;
use crate::protocol::{self, UiResponse};
use crate::source_control::{
    run_source_control_action, source_control_diff, source_control_image_diff,
    source_control_status, SourceControlLocks,
};

use super::request_cancellation::RequestCancellation;
use super::{repo_commands, ConnectionContext};

pub async fn source_control_status_command(
    command: protocol::SourceControlStatusCommand,
    ctx: &ConnectionContext,
    cancellation: &RequestCancellation,
) -> Result<UiResponse, AgentError> {
    let (repo_root, locks) = command_repo(&command.repo_id, ctx).await?;
    let status = source_control_status(
        Path::new(&repo_root),
        Some(cancellation.source_control_read_token()?),
        &locks,
    )
    .await?;
    Ok(UiResponse::SourceControlStatusResult(
        protocol::SourceControlStatusResult {
            repo_id: command.repo_id,
            status,
        },
    ))
}

pub async fn source_control_diff_command(
    command: protocol::SourceControlDiffCommand,
    ctx: &ConnectionContext,
    cancellation: &RequestCancellation,
) -> Result<UiResponse, AgentError> {
    let (repo_root, _locks) = command_repo(&command.repo_id, ctx).await?;
    let diff = source_control_diff(
        Path::new(&repo_root),
        &command.path,
        command.original_path.as_deref(),
        command.area,
        Some(cancellation.source_control_read_token()?),
    )
    .await?;
    Ok(UiResponse::SourceControlDiffResult(
        protocol::SourceControlDiffResult {
            repo_id: command.repo_id,
            path: command.path,
            area: command.area,
            patch: diff.patch,
            truncated: diff.truncated,
            binary: diff.binary,
        },
    ))
}

pub async fn source_control_image_diff_command(
    command: protocol::SourceControlImageDiffCommand,
    ctx: &ConnectionContext,
    cancellation: &RequestCancellation,
) -> Result<UiResponse, AgentError> {
    let (repo_root, _locks) = command_repo(&command.repo_id, ctx).await?;
    let diff = source_control_image_diff(
        Path::new(&repo_root),
        &command.path,
        command.original_path.as_deref(),
        command.area,
        Some(cancellation.source_control_read_token()?),
    )
    .await?;
    Ok(UiResponse::SourceControlImageDiffResult(
        protocol::SourceControlImageDiffResult {
            repo_id: command.repo_id,
            path: command.path,
            area: command.area,
            before: diff.before,
            after: diff.after,
        },
    ))
}

pub async fn source_control_action_command(
    command: protocol::SourceControlActionCommand,
    ctx: &ConnectionContext,
) -> Result<UiResponse, AgentError> {
    let (repo_root, locks) = command_repo(&command.repo_id, ctx).await?;
    let kind = command.action.kind();
    let outcome =
        run_source_control_action(&locks, Path::new(&repo_root), command.action).await?;
    Ok(UiResponse::SourceControlActionResult(
        protocol::SourceControlActionResult {
            repo_id: command.repo_id,
            kind,
            status: outcome.status,
            commit_sha: outcome.commit_sha,
            hook_changed_paths: outcome.hook_changed_paths,
        },
    ))
}

/// The configured root and the mutation-lock registry, cloned out under one
/// short read lock so no runtime lock is held across Git.
async fn command_repo(
    repo_id: &str,
    ctx: &ConnectionContext,
) -> Result<(String, SourceControlLocks), AgentError> {
    let state = ctx.runtime.read().await;
    let repo_config = state
        .repo_configs
        .get(repo_id)
        .ok_or_else(|| AgentError::new("UNKNOWN_REPO", format!("Unknown repo: {repo_id}")))?;
    let repo_root = repo_commands::resolve_wsl_repo_root(repo_id, repo_config)?;
    Ok((repo_root, state.source_control_locks.clone()))
}
