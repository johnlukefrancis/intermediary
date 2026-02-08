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
        staging_host_root: source_command.staging_host_root.clone(),
        staging_wsl_root: source_command.staging_wsl_root.clone(),
        auto_stage_on_change: source_command.auto_stage_on_change,
    })
}

pub(super) fn client_hello_fingerprint(command: &ClientHelloCommand) -> Result<String, AgentError> {
    serde_json::to_string(command).map_err(|err| AgentError::new("INVALID_CONFIG", err.to_string()))
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

pub(super) fn should_forward_wsl_hello(had_wsl_before: bool, has_wsl_now: bool) -> bool {
    has_wsl_now || had_wsl_before
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{client_hello_fingerprint, should_forward_wsl_hello};
    use im_agent::protocol::ClientHelloCommand;

    #[test]
    fn forwards_when_config_still_has_wsl_repos() {
        assert!(should_forward_wsl_hello(true, true));
        assert!(should_forward_wsl_hello(false, true));
    }

    #[test]
    fn forwards_empty_hello_when_wsl_repos_removed() {
        assert!(should_forward_wsl_hello(true, false));
    }

    #[test]
    fn skips_wsl_hello_when_no_wsl_repos_ever_configured() {
        assert!(!should_forward_wsl_hello(false, false));
    }

    #[test]
    fn fingerprint_changes_when_payload_changes() {
        let baseline = ClientHelloCommand {
            config: json!({"repos": [{"repoId": "a", "root": {"kind": "wsl", "path": "/repo"}}]}),
            staging_host_root: "C:\\staging".to_string(),
            staging_wsl_root: Some("/mnt/c/staging".to_string()),
            auto_stage_on_change: Some(false),
        };
        let mut changed = baseline.clone();
        changed.auto_stage_on_change = Some(true);

        let baseline_fp = client_hello_fingerprint(&baseline).expect("baseline fingerprint");
        let changed_fp = client_hello_fingerprint(&changed).expect("changed fingerprint");
        assert_ne!(baseline_fp, changed_fp);
    }
}
