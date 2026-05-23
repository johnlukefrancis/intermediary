// Path: crates/im_agent/src/server/connection/repo_commands.rs
// Description: Repo file-read and topology command handlers for WebSocket dispatch

use crate::error::AgentError;
use crate::protocol::{self, UiResponse};
use crate::repos::{get_repo_top_level, list_repo_directory, read_image_file, read_text_file};

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
