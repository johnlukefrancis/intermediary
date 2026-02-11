// Path: crates/im_agent/src/runtime/state.rs
// Description: Agent runtime state and option handlers

use std::collections::HashMap;
use std::path::PathBuf;

use serde_json::Value;

use crate::error::AgentError;
use crate::logging::Logger;
use crate::protocol::{
    AgentErrorDetails, AgentErrorEvent, AgentEvent, ClientHelloCommand, ClientHelloResult,
    RefreshResult, SetOptionsCommand, SetOptionsResult, WatchRepoResult,
};
use crate::repos::{is_valid_repo_root, RecentFilesStore, RepoWatcher};
use crate::server::EventBus;
use crate::staging::PathBridgeConfig;

use super::{compute_config_fingerprint, AppConfig, RepoConfig, RepoRootKind};

pub struct AgentRuntime {
    pub supported_root_kind: RepoRootKind,
    pub config: Option<AppConfig>,
    pub config_fingerprint: Option<String>,
    pub staging: Option<PathBridgeConfig>,
    pub repo_configs: HashMap<String, RepoConfig>,
    pub recent_files_store: Option<RecentFilesStore>,
    pub watchers: HashMap<String, RepoWatcher>,
    pub auto_stage_on_change: bool,
    pub recent_files_limit: usize,
}

impl AgentRuntime {
    pub fn new() -> Self {
        Self::new_for_root_kind(RepoRootKind::Wsl)
    }

    pub fn new_for_root_kind(supported_root_kind: RepoRootKind) -> Self {
        Self {
            supported_root_kind,
            config: None,
            config_fingerprint: None,
            staging: None,
            repo_configs: HashMap::new(),
            recent_files_store: None,
            watchers: HashMap::new(),
            auto_stage_on_change: true,
            recent_files_limit: 40,
        }
    }

    pub async fn apply_client_hello(
        &mut self,
        command: ClientHelloCommand,
        agent_version: &str,
        event_bus: &EventBus,
        logger: &Logger,
    ) -> Result<ClientHelloResult, AgentError> {
        let resolved_auto_stage = resolve_auto_stage(&command, self.auto_stage_on_change);

        let parsed_config = parse_app_config(&command.config)?;
        let staging_root_for_runtime = self.staging_root_for_runtime(&command)?;
        let fingerprint = compute_config_fingerprint(&parsed_config, &staging_root_for_runtime);
        let needs_reset = self
            .config_fingerprint
            .as_ref()
            .map(|current| current != &fingerprint)
            .unwrap_or(true);

        if needs_reset {
            self.reset_watchers().await;
        }

        self.config_fingerprint = Some(fingerprint);
        self.config = Some(parsed_config.clone());
        self.staging = Some(PathBridgeConfig {
            staging_host_root: command.staging_host_root.clone(),
            staging_wsl_root: command.staging_wsl_root.clone(),
        });
        self.auto_stage_on_change = resolved_auto_stage;
        self.recent_files_limit = parsed_config.recent_files_limit;

        self.repo_configs = parsed_config
            .repos
            .iter()
            .cloned()
            .map(|repo| (repo.repo_id.clone(), repo))
            .collect();

        if needs_reset || self.recent_files_store.is_none() {
            let state_dir = PathBuf::from(&staging_root_for_runtime).join("state");
            self.recent_files_store = Some(RecentFilesStore::new(state_dir, logger.clone()));
        }

        for repo in parsed_config.repos.iter() {
            let Some(repo_root) = repo.root.path_for_kind(self.supported_root_kind) else {
                logger.info(
                    "Skipping unsupported repo root for runtime",
                    Some(serde_json::json!({
                        "repoId": repo.repo_id,
                        "supportedRootKind": self.supported_root_kind.as_str(),
                        "rootKind": repo.root.kind(),
                        "rootPath": repo.root.path()
                    })),
                );
                if self.supported_root_kind == RepoRootKind::Host
                    && repo.root_kind() == RepoRootKind::Wsl
                    && cfg!(not(target_os = "windows"))
                {
                    event_bus.broadcast_event(AgentEvent::Error(AgentErrorEvent::new(
                        "config",
                        format!(
                            "WSL repo root not supported on this platform: {}",
                            repo.repo_id
                        ),
                        Some(AgentErrorDetails {
                            code: None,
                            doc_path: None,
                            repo_id: Some(repo.repo_id.clone()),
                            raw_code: Some("UNSUPPORTED_REPO_ROOT".to_string()),
                            raw_message: None,
                        }),
                    )));
                }
                continue;
            };
            if !is_valid_repo_root(repo_root).await {
                logger.warn(
                    "Invalid repo root, skipping watcher",
                    Some(serde_json::json!({"repoId": repo.repo_id, "rootPath": repo_root})),
                );
                continue;
            }
            if let Err(err) = self
                .ensure_repo_watcher_running(repo, event_bus, logger)
                .await
            {
                logger.error(
                    "Failed to start repo watcher",
                    Some(serde_json::json!({"repoId": repo.repo_id, "error": err.message()})),
                );
            }
        }

        Ok(ClientHelloResult {
            agent_version: agent_version.to_string(),
            watched_repo_ids: self.watched_repo_ids(),
        })
    }

    pub fn apply_set_options(&mut self, command: SetOptionsCommand) -> SetOptionsResult {
        if let Some(value) = command.auto_stage_on_change {
            self.auto_stage_on_change = value;
        }

        SetOptionsResult {
            auto_stage_on_change: self.auto_stage_on_change,
        }
    }

    pub async fn watch_repo(
        &mut self,
        repo_id: &str,
        event_bus: &EventBus,
        logger: &Logger,
    ) -> Result<WatchRepoResult, AgentError> {
        if self
            .watchers
            .get(repo_id)
            .map(|watcher| !watcher.is_task_finished())
            .unwrap_or(false)
        {
            return Ok(WatchRepoResult {
                repo_id: repo_id.to_string(),
            });
        }

        let repo_config =
            self.repo_configs.get(repo_id).cloned().ok_or_else(|| {
                AgentError::new("UNKNOWN_REPO", format!("Unknown repo: {repo_id}"))
            })?;

        let repo_root = repo_config
            .root_path_for_kind(self.supported_root_kind)
            .ok_or_else(|| {
                AgentError::new(
                    "UNSUPPORTED_REPO_ROOT",
                    format!(
                        "Repo {repo_id} root kind {} is unsupported by {} runtime",
                        repo_config.root.kind(),
                        self.supported_root_kind.as_str()
                    ),
                )
            })?;

        if !is_valid_repo_root(repo_root).await {
            return Err(AgentError::new(
                "INVALID_REPO",
                format!("Invalid repo root: {repo_root}"),
            ));
        }

        self.ensure_repo_watcher_running(&repo_config, event_bus, logger)
            .await?;

        Ok(WatchRepoResult {
            repo_id: repo_id.to_string(),
        })
    }

    pub async fn refresh_repo(&self, repo_id: &str) -> Result<RefreshResult, AgentError> {
        let watcher = self.watchers.get(repo_id).ok_or_else(|| {
            AgentError::new("UNKNOWN_REPO", format!("Repo not watched: {repo_id}"))
        })?;
        watcher.broadcast_snapshot();
        Ok(RefreshResult {
            repo_id: repo_id.to_string(),
        })
    }

    fn staging_root_for_runtime(&self, command: &ClientHelloCommand) -> Result<String, AgentError> {
        match self.supported_root_kind {
            RepoRootKind::Wsl => command.staging_wsl_root.clone().ok_or_else(|| {
                AgentError::new(
                    "MISSING_WSL_ROOT",
                    "Missing stagingWslRoot for WSL runtime in clientHello",
                )
            }),
            RepoRootKind::Host => Ok(command.staging_host_root.clone()),
        }
    }
}

fn resolve_auto_stage(command: &ClientHelloCommand, fallback: bool) -> bool {
    if let Some(value) = command.auto_stage_on_change {
        return value;
    }

    command
        .config
        .get("autoStageGlobal")
        .and_then(|value| value.as_bool())
        .unwrap_or(fallback)
}

fn parse_app_config(config: &Value) -> Result<AppConfig, AgentError> {
    serde_json::from_value(config.clone())
        .map_err(|err| AgentError::new("INVALID_CONFIG", err.to_string()))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn wsl_runtime_requires_staging_wsl_root() {
        let runtime = AgentRuntime::new_for_root_kind(RepoRootKind::Wsl);
        let command = ClientHelloCommand {
            config: json!({}),
            staging_host_root: "C:\\staging".to_string(),
            staging_wsl_root: None,
            auto_stage_on_change: None,
        };

        let err = runtime
            .staging_root_for_runtime(&command)
            .expect_err("missing stagingWslRoot should fail");
        assert_eq!(err.code(), "MISSING_WSL_ROOT");
    }

    #[test]
    fn wsl_runtime_uses_wsl_staging_root() {
        let runtime = AgentRuntime::new_for_root_kind(RepoRootKind::Wsl);
        let command = ClientHelloCommand {
            config: json!({}),
            staging_host_root: "C:\\staging".to_string(),
            staging_wsl_root: Some("/mnt/c/staging".to_string()),
            auto_stage_on_change: None,
        };

        let root = runtime
            .staging_root_for_runtime(&command)
            .expect("stagingWslRoot should be accepted");
        assert_eq!(root, "/mnt/c/staging");
    }
}
