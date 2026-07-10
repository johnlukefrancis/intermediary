// Path: crates/im_agent/src/runtime/watcher_reconciliation.rs
// Description: Concurrent repository watcher reconciliation for agent clientHello bootstrap

use futures_util::future::join_all;
use serde_json::json;

use crate::error::AgentError;
use crate::logging::Logger;
use crate::protocol::{AgentErrorDetails, AgentErrorEvent, AgentEvent};
use crate::repos::{is_valid_repo_root, RepoWatcher, RepoWatcherConfig};
use crate::server::EventBus;

use super::state::AgentRuntime;
use super::{RepoConfig, RepoRootKind};

impl AgentRuntime {
    pub(crate) async fn reconcile_repo_watchers(
        &mut self,
        repos: &[RepoConfig],
        event_bus: &EventBus,
        logger: &Logger,
    ) {
        let mut pending = Vec::new();
        for repo in repos {
            let Some(repo_root) = repo.root.path_for_kind(self.supported_root_kind) else {
                self.report_unsupported_repo(repo, event_bus, logger);
                continue;
            };
            if !is_valid_repo_root(repo_root).await {
                logger.warn(
                    "Invalid repo root, skipping watcher",
                    Some(json!({"repoId": repo.repo_id, "rootPath": repo_root})),
                );
                continue;
            }
            match self
                .prepare_repo_watcher_start(repo, event_bus, logger)
                .await
            {
                Ok(Some(config)) => pending.push(config),
                Ok(None) => {}
                Err(err) => log_watcher_start_error(&repo.repo_id, &err, logger),
            }
        }

        let starts = pending.into_iter().map(|config| async move {
            let repo_id = config.repo_id.clone();
            (repo_id, RepoWatcher::start(config).await)
        });
        for (repo_id, result) in join_all(starts).await {
            match result {
                Ok(watcher) => {
                    self.watchers.insert(repo_id, watcher);
                }
                Err(err) => log_watcher_start_error(&repo_id, &err, logger),
            }
        }
    }

    async fn prepare_repo_watcher_start(
        &mut self,
        repo: &RepoConfig,
        event_bus: &EventBus,
        logger: &Logger,
    ) -> Result<Option<RepoWatcherConfig>, AgentError> {
        if self
            .watchers
            .get(&repo.repo_id)
            .is_some_and(RepoWatcher::is_task_finished)
        {
            logger.warn(
                "Repo watcher task ended; restarting watcher",
                Some(json!({"repoId": repo.repo_id})),
            );
            if let Some(watcher) = self.watchers.remove(&repo.repo_id) {
                watcher.stop().await;
            }
        }
        if self.watchers.contains_key(&repo.repo_id) {
            return Ok(None);
        }
        self.repo_watcher_config(repo, event_bus, logger).map(Some)
    }

    fn report_unsupported_repo(&self, repo: &RepoConfig, event_bus: &EventBus, logger: &Logger) {
        logger.info(
            "Skipping unsupported repo root for runtime",
            Some(json!({
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
    }
}

fn log_watcher_start_error(repo_id: &str, err: &AgentError, logger: &Logger) {
    logger.error(
        "Failed to start repo watcher",
        Some(json!({"repoId": repo_id, "error": err.message()})),
    );
}
