// Path: crates/im_agent/src/runtime/state.rs
// Description: Agent runtime state and option handlers

use std::collections::HashMap;
use std::path::PathBuf;

use serde_json::Value;

use crate::error::AgentError;
use crate::logging::Logger;
use crate::protocol::{
    ClientHelloCommand, ClientHelloResult, RefreshResult, SetOptionsCommand, SetOptionsResult,
    WatchRepoResult,
};
use crate::repos::{is_valid_repo_root, RecentFilesStore, RepoWatcher, RepoWatcherConfig};
use crate::server::EventBus;
use crate::staging::PathBridgeConfig;

use super::{compute_config_fingerprint, AppConfig, RepoConfig};

pub struct AgentRuntime {
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
        Self {
            config: None,
            config_fingerprint: None,
            staging: None,
            repo_configs: HashMap::new(),
            recent_files_store: None,
            watchers: HashMap::new(),
            auto_stage_on_change: true,
            recent_files_limit: 200,
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
        let fingerprint = compute_config_fingerprint(&parsed_config, &command.staging_wsl_root);
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
            staging_wsl_root: command.staging_wsl_root,
            staging_win_root: command.staging_win_root,
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
            let state_dir =
                PathBuf::from(&self.staging.as_ref().unwrap().staging_wsl_root).join("state");
            self.recent_files_store = Some(RecentFilesStore::new(state_dir, logger.clone()));
        }

        for repo in parsed_config.repos.iter() {
            if self.watchers.contains_key(&repo.repo_id) {
                continue;
            }
            let Some(repo_root) = repo.root.wsl_path() else {
                logger.info(
                    "Skipping non-WSL repo root",
                    Some(serde_json::json!({
                        "repoId": repo.repo_id,
                        "rootKind": repo.root.kind(),
                        "rootPath": repo.root.path()
                    })),
                );
                continue;
            };
            if !is_valid_repo_root(repo_root).await {
                logger.warn(
                    "Invalid repo root, skipping watcher",
                    Some(serde_json::json!({"repoId": repo.repo_id, "rootPath": repo_root})),
                );
                continue;
            }
            if let Err(err) = self.start_repo_watcher(repo, event_bus, logger).await {
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
        if self.watchers.contains_key(repo_id) {
            return Ok(WatchRepoResult {
                repo_id: repo_id.to_string(),
            });
        }

        let repo_config =
            self.repo_configs.get(repo_id).cloned().ok_or_else(|| {
                AgentError::new("UNKNOWN_REPO", format!("Unknown repo: {repo_id}"))
            })?;

        let repo_root = repo_config.wsl_root_path().ok_or_else(|| {
            AgentError::new(
                "UNSUPPORTED_REPO_ROOT",
                format!(
                    "Repo {repo_id} uses unsupported root kind: {}",
                    repo_config.root.kind()
                ),
            )
        })?;

        if !is_valid_repo_root(repo_root).await {
            return Err(AgentError::new(
                "INVALID_REPO",
                format!("Invalid repo root: {repo_root}"),
            ));
        }

        self.start_repo_watcher(&repo_config, event_bus, logger)
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

    pub async fn reset_watchers(&mut self) {
        for watcher in self.watchers.values() {
            watcher.stop().await;
        }
        self.watchers.clear();

        if let Some(store) = &self.recent_files_store {
            store.flush_all().await;
        }
    }

    fn watched_repo_ids(&self) -> Vec<String> {
        self.watchers.keys().cloned().collect()
    }

    async fn start_repo_watcher(
        &mut self,
        repo: &RepoConfig,
        event_bus: &EventBus,
        logger: &Logger,
    ) -> Result<(), AgentError> {
        let store = self.recent_files_store.as_ref().ok_or_else(|| {
            AgentError::new("NOT_CONFIGURED", "Recent files store not configured")
        })?;

        let repo_root = repo.wsl_root_path().ok_or_else(|| {
            AgentError::new(
                "UNSUPPORTED_REPO_ROOT",
                format!(
                    "Repo {} uses unsupported root kind: {}",
                    repo.repo_id,
                    repo.root.kind()
                ),
            )
        })?;

        let initial_entries = store.load(&repo.repo_id, repo_root).await;

        let watcher = RepoWatcher::start(RepoWatcherConfig {
            repo_id: repo.repo_id.clone(),
            root_path: repo_root.to_string(),
            docs_globs: repo.docs_globs.clone(),
            code_globs: repo.code_globs.clone(),
            ignore_globs: repo.ignore_globs.clone(),
            mru_capacity: self.recent_files_limit.max(1),
            initial_entries,
            recent_store: store.clone(),
            logger: logger.clone(),
            event_bus: event_bus.clone(),
        })
        .await?;

        self.watchers.insert(repo.repo_id.clone(), watcher);
        Ok(())
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
