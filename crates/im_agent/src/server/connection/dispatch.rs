// Path: crates/im_agent/src/server/connection/dispatch.rs
// Description: Command dispatch for WebSocket request handling

use crate::bundles::{build_bundle, list_bundles, BuildBundleOptions, ListBundlesOptions};
use crate::error::AgentError;
use crate::protocol::{BundleBuiltEvent, BundleInfo, UiCommand, UiResponse};
use crate::repos::get_repo_top_level;
use crate::staging::stage_file;

use super::ConnectionContext;

pub async fn dispatch_command(
    command: UiCommand,
    ctx: &ConnectionContext,
) -> Result<UiResponse, AgentError> {
    match command {
        UiCommand::ClientHello(command) => {
            let mut state = ctx.runtime.write().await;
            let result = state
                .apply_client_hello(command, &ctx.agent_version, &ctx.event_bus, &ctx.logger)
                .await?;
            Ok(UiResponse::ClientHelloResult(result))
        }
        UiCommand::SetOptions(command) => {
            let mut state = ctx.runtime.write().await;
            let result = state.apply_set_options(command);
            Ok(UiResponse::SetOptionsResult(result))
        }
        UiCommand::WatchRepo(command) => {
            let mut state = ctx.runtime.write().await;
            let result = state
                .watch_repo(&command.repo_id, &ctx.event_bus, &ctx.logger)
                .await?;
            Ok(UiResponse::WatchRepoResult(result))
        }
        UiCommand::Refresh(command) => {
            let state = ctx.runtime.read().await;
            let result = state.refresh_repo(&command.repo_id).await?;
            Ok(UiResponse::RefreshResult(result))
        }
        UiCommand::StageFile(command) => {
            let (staging, repo_root) = {
                let state = ctx.runtime.read().await;
                let staging = state
                    .staging
                    .clone()
                    .ok_or_else(|| AgentError::new("NOT_CONFIGURED", "Staging not configured"))?;
                let repo_config = state.repo_configs.get(&command.repo_id).ok_or_else(|| {
                    AgentError::new("UNKNOWN_REPO", format!("Unknown repo: {}", command.repo_id))
                })?;
                (staging, resolve_wsl_repo_root(&command.repo_id, repo_config)?)
            };

            let result = stage_file(&staging, &command.repo_id, &repo_root, &command.path).await?;
            Ok(UiResponse::StageFileResult(
                crate::protocol::StageFileResult {
                    repo_id: command.repo_id,
                    path: command.path,
                    windows_path: result.windows_path,
                    wsl_path: result.wsl_path,
                    bytes_copied: result.bytes_copied,
                    mtime_ms: result.mtime_ms,
                },
            ))
        }
        UiCommand::BuildBundle(command) => {
            let (staging, repo_config) = {
                let state = ctx.runtime.read().await;
                let staging = state
                    .staging
                    .clone()
                    .ok_or_else(|| AgentError::new("NOT_CONFIGURED", "Staging not configured"))?;
                let repo_config = state
                    .repo_configs
                    .get(&command.repo_id)
                    .cloned()
                    .ok_or_else(|| {
                        AgentError::new(
                            "UNKNOWN_REPO",
                            format!("Unknown repo: {}", command.repo_id),
                        )
                    })?;
                (staging, repo_config)
            };

            let preset = repo_config
                .bundle_presets
                .iter()
                .find(|preset| preset.preset_id == command.preset_id)
                .ok_or_else(|| {
                    AgentError::new(
                        "UNKNOWN_PRESET",
                        format!("Unknown preset: {}", command.preset_id),
                    )
                })?
                .clone();
            let repo_root = resolve_wsl_repo_root(&command.repo_id, &repo_config)?;

            let result = build_bundle(
                BuildBundleOptions {
                    repo_id: command.repo_id.clone(),
                    repo_root,
                    preset_id: command.preset_id.clone(),
                    preset_name: preset.preset_name,
                    selection: command.selection,
                    staging_wsl_root: staging.staging_wsl_root,
                    global_excludes: command.global_excludes,
                },
                &ctx.event_bus,
                &ctx.logger,
            )
            .await?;

            ctx.event_bus
                .broadcast_event(crate::protocol::AgentEvent::BundleBuilt(BundleBuiltEvent {
                    repo_id: command.repo_id.clone(),
                    preset_id: command.preset_id.clone(),
                    windows_path: result.windows_path.clone(),
                    alias_windows_path: result.alias_windows_path.clone(),
                    bytes: result.bytes,
                    file_count: result.file_count,
                    built_at_iso: result.built_at_iso.clone(),
                }));

            Ok(UiResponse::BuildBundleResult(
                crate::protocol::BuildBundleResult {
                    repo_id: command.repo_id,
                    preset_id: command.preset_id,
                    windows_path: result.windows_path,
                    wsl_path: result.wsl_path,
                    alias_windows_path: result.alias_windows_path,
                    bytes: result.bytes,
                    file_count: result.file_count,
                    built_at_iso: result.built_at_iso,
                },
            ))
        }
        UiCommand::GetRepoTopLevel(command) => {
            let repo_root = {
                let state = ctx.runtime.read().await;
                let repo_config = state.repo_configs.get(&command.repo_id).ok_or_else(|| {
                    AgentError::new("UNKNOWN_REPO", format!("Unknown repo: {}", command.repo_id))
                })?;
                resolve_wsl_repo_root(&command.repo_id, repo_config)?
            };

            let result = get_repo_top_level(&repo_root)
                .await
                .map_err(|err| AgentError::internal(format!("Failed to scan repo: {err}")))?;

            Ok(UiResponse::GetRepoTopLevelResult(
                crate::protocol::GetRepoTopLevelResult {
                    repo_id: command.repo_id,
                    dirs: result.dirs,
                    files: result.files,
                    subdirs: Some(result.subdirs),
                },
            ))
        }
        UiCommand::ListBundles(command) => {
            let staging_root = {
                let state = ctx.runtime.read().await;
                state
                    .staging
                    .as_ref()
                    .map(|staging| staging.staging_wsl_root.clone())
                    .ok_or_else(|| AgentError::new("NOT_CONFIGURED", "Staging not configured"))?
            };

            let bundles = list_bundles(ListBundlesOptions {
                staging_wsl_root: staging_root,
                repo_id: command.repo_id.clone(),
                preset_id: command.preset_id.clone(),
            })
            .await?;
            let results: Vec<BundleInfo> = bundles;

            Ok(UiResponse::ListBundlesResult(
                crate::protocol::ListBundlesResult {
                    repo_id: command.repo_id,
                    preset_id: command.preset_id,
                    bundles: results,
                },
            ))
        }
        UiCommand::Unknown => Err(AgentError::new("UNKNOWN_COMMAND", "Unsupported command")),
    }
}

fn resolve_wsl_repo_root(
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
