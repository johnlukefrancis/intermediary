// Path: crates/im_agent/src/server/connection/repo_commands.rs
// Description: Repo file-read and topology command handlers for WebSocket dispatch

use std::path::Path;

use crate::error::AgentError;
use crate::protocol::{self, UiResponse};
use crate::repos::worktree::worktree_action;
use crate::repos::{
    get_repo_top_level, import_files, list_repo_directory, read_image_file, read_text_file,
};
use crate::source_control::SourceControlLocks;
use crate::staging::{StageFileCancelToken, StagingRootKind};

use super::ConnectionContext;

pub async fn read_text_file_command(
    command: protocol::ReadTextFileCommand,
    ctx: &ConnectionContext,
) -> Result<UiResponse, AgentError> {
    let repo_root = command_repo_root(&command.repo_id, ctx).await?;
    let result = read_text_file(&repo_root, &command.path).await?;
    Ok(UiResponse::ReadTextFileResult(
        protocol::ReadTextFileResult {
            repo_id: command.repo_id,
            path: command.path,
            content: result.content,
            bytes: result.bytes,
            mtime_ms: result.mtime_ms,
            encoding: "utf-8".to_string(),
        },
    ))
}

pub async fn read_image_file_command(
    command: protocol::ReadImageFileCommand,
    ctx: &ConnectionContext,
) -> Result<UiResponse, AgentError> {
    let repo_root = command_repo_root(&command.repo_id, ctx).await?;
    let result = read_image_file(&repo_root, &command.path).await?;
    Ok(UiResponse::ReadImageFileResult(
        protocol::ReadImageFileResult {
            repo_id: command.repo_id,
            path: command.path,
            data_base64: result.data_base64,
            mime_type: result.mime_type,
            bytes: result.bytes,
            mtime_ms: result.mtime_ms,
        },
    ))
}

pub async fn get_repo_top_level_command(
    command: protocol::GetRepoTopLevelCommand,
    ctx: &ConnectionContext,
) -> Result<UiResponse, AgentError> {
    let repo_root = command_repo_root(&command.repo_id, ctx).await?;
    let result = get_repo_top_level(&repo_root)
        .await
        .map_err(|err| AgentError::internal(format!("Failed to scan repo: {err}")))?;

    Ok(UiResponse::GetRepoTopLevelResult(
        protocol::GetRepoTopLevelResult {
            repo_id: command.repo_id,
            dirs: result.dirs,
            files: result.files,
            subdirs: Some(result.subdirs),
            default_excluded: result.default_excluded,
        },
    ))
}

pub async fn list_repo_directory_command(
    command: protocol::ListRepoDirectoryCommand,
    ctx: &ConnectionContext,
) -> Result<UiResponse, AgentError> {
    let repo_root = command_repo_root(&command.repo_id, ctx).await?;
    let result = list_repo_directory(&repo_root, &command.path).await?;

    Ok(UiResponse::ListRepoDirectoryResult(
        protocol::ListRepoDirectoryResult {
            repo_id: command.repo_id,
            path: result.path,
            dirs: result.dirs,
            files: result.files,
        },
    ))
}

/// Copies external OS files into one directory of a WSL-rooted repo.
///
/// An import writes the worktree without Git, so it takes the same
/// per-worktree mutation lock every Git mutation takes: a drop must not
/// interleave with a commit or a discard over the same index. The lock is also
/// the drain gate, so a shutdown refuses new imports with `AGENT_DRAINING`.
/// Resolving the lock spawns `rev-parse`, so a configured root that is not a
/// repository is refused there (`GIT_NOT_REPOSITORY`) rather than being given
/// an unlocked path of its own.
pub async fn import_files_command(
    command: protocol::ImportFilesCommand,
    ctx: &ConnectionContext,
) -> Result<UiResponse, AgentError> {
    let (repo_root, locks) = command_repo(&command.repo_id, ctx).await?;
    let root = Path::new(&repo_root);
    let _guard = locks.acquire(root).await?;

    let imported = import_files(
        root,
        &command.directory,
        &command.sources,
        &command.on_conflict,
        StagingRootKind::Wsl,
        &StageFileCancelToken::new(),
    )
    .await?;

    Ok(UiResponse::ImportFilesResult(protocol::ImportFilesResult {
        repo_id: command.repo_id,
        directory: command.directory,
        imported,
    }))
}

/// Deletes, moves, copies, or renames entries in a WSL-rooted repo worktree.
///
/// Like an import, this writes the worktree without Git and therefore takes
/// the same per-worktree mutation lock every Git mutation takes: a rename must
/// not interleave with a commit or a discard over the same index. The lock is
/// also the drain gate, so a shutdown refuses new actions with
/// `AGENT_DRAINING`, and resolving it spawns `rev-parse`, so a configured root
/// that is not a repository is refused there rather than being given an
/// unlocked path of its own.
pub async fn worktree_action_command(
    command: protocol::WorktreeActionCommand,
    ctx: &ConnectionContext,
) -> Result<UiResponse, AgentError> {
    let (repo_root, locks) = command_repo(&command.repo_id, ctx).await?;
    let root = Path::new(&repo_root);
    let _guard = locks.acquire(root).await?;

    let kind = command.action.kind();
    let entries = worktree_action(
        root,
        &command.action,
        &locks,
        &StageFileCancelToken::new(),
    )
    .await?;

    Ok(UiResponse::WorktreeActionResult(
        protocol::WorktreeActionResult {
            repo_id: command.repo_id,
            kind,
            entries,
        },
    ))
}

pub(super) fn resolve_wsl_repo_root(
    repo_id: &str,
    repo_config: &crate::runtime::RepoConfig,
) -> Result<String, AgentError> {
    repo_config
        .wsl_root_path()
        .map(str::to_string)
        .ok_or_else(|| {
            AgentError::new(
                "UNSUPPORTED_REPO_ROOT",
                format!(
                    "Repo {repo_id} uses unsupported root kind: {}",
                    repo_config.root.kind()
                ),
            )
        })
}

async fn command_repo_root(repo_id: &str, ctx: &ConnectionContext) -> Result<String, AgentError> {
    let state = ctx.runtime.read().await;
    let repo_config = state
        .repo_configs
        .get(repo_id)
        .ok_or_else(|| AgentError::new("UNKNOWN_REPO", format!("Unknown repo: {repo_id}")))?;
    resolve_wsl_repo_root(repo_id, repo_config)
}

/// The configured root and the mutation-lock registry, cloned out under one
/// short read lock so no runtime lock is held across the copy.
async fn command_repo(
    repo_id: &str,
    ctx: &ConnectionContext,
) -> Result<(String, SourceControlLocks), AgentError> {
    let state = ctx.runtime.read().await;
    let repo_config = state
        .repo_configs
        .get(repo_id)
        .ok_or_else(|| AgentError::new("UNKNOWN_REPO", format!("Unknown repo: {repo_id}")))?;
    let repo_root = resolve_wsl_repo_root(repo_id, repo_config)?;
    Ok((repo_root, state.source_control_locks.clone()))
}
