// Path: crates/im_host_agent/src/runtime/host_runtime_helpers.rs
// Description: Host-runtime helper functions for config parsing and repo-command metadata

use std::collections::HashMap;

use im_agent::error::AgentError;
use im_agent::protocol::{ClientHelloCommand, UiCommand};
use im_agent::runtime::{AppConfig, RepoRootKind};

use super::repo_backend::RepoBackend;

pub(super) fn build_repo_backend_map(config: &AppConfig) -> HashMap<String, RepoBackend> {
    config
        .repos
        .iter()
        .map(|repo| (repo.repo_id.clone(), RepoBackend::from_repo_config(repo)))
        .collect()
}

pub(super) fn build_wsl_client_hello(
    parsed_config: &AppConfig,
    source_command: &ClientHelloCommand,
) -> Result<ClientHelloCommand, AgentError> {
    let mut wsl_config = parsed_config.clone();
    wsl_config
        .repos
        .retain(|repo| repo.root_kind() == RepoRootKind::Wsl);

    let config = serde_json::to_value(wsl_config)
        .map_err(|err| AgentError::new("INVALID_CONFIG", err.to_string()))?;

    Ok(ClientHelloCommand {
        config,
        staging_wsl_root: source_command.staging_wsl_root.clone(),
        staging_win_root: source_command.staging_win_root.clone(),
        auto_stage_on_change: source_command.auto_stage_on_change,
    })
}

pub(super) fn parse_app_config(config: &serde_json::Value) -> Result<AppConfig, AgentError> {
    serde_json::from_value(config.clone())
        .map_err(|err| AgentError::new("INVALID_CONFIG", err.to_string()))
}

pub(super) fn repo_id_from_command(command: &UiCommand) -> Option<&str> {
    match command {
        UiCommand::WatchRepo(command) => Some(&command.repo_id),
        UiCommand::Refresh(command) => Some(&command.repo_id),
        UiCommand::StageFile(command) => Some(&command.repo_id),
        UiCommand::BuildBundle(command) => Some(&command.repo_id),
        UiCommand::GetRepoTopLevel(command) => Some(&command.repo_id),
        UiCommand::ListBundles(command) => Some(&command.repo_id),
        UiCommand::ClientHello(_) | UiCommand::SetOptions(_) | UiCommand::Unknown => None,
    }
}
