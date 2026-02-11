// Path: crates/im_agent/src/runtime/state_watchers.rs
// Description: Watcher lifecycle helpers for agent runtime state

use crate::error::AgentError;
use crate::logging::Logger;
use crate::repos::{RepoWatcher, RepoWatcherConfig};
use crate::server::EventBus;
use serde_json::json;

use super::state::AgentRuntime;
use super::RepoConfig;

impl AgentRuntime {
    pub async fn reset_watchers(&mut self) {
        for watcher in self.watchers.values() {
            watcher.stop().await;
        }
        self.watchers.clear();

        if let Some(store) = &self.recent_files_store {
            store.flush_all().await;
        }
    }

    pub(crate) fn watched_repo_ids(&self) -> Vec<String> {
        self.watchers
            .iter()
            .filter_map(|(repo_id, watcher)| {
                if watcher.is_task_finished() {
                    return None;
                }
                Some(repo_id.clone())
            })
            .collect()
    }

    pub(crate) async fn start_repo_watcher(
        &mut self,
        repo: &RepoConfig,
        event_bus: &EventBus,
        logger: &Logger,
    ) -> Result<(), AgentError> {
        let store = self.recent_files_store.as_ref().ok_or_else(|| {
            AgentError::new("NOT_CONFIGURED", "Recent files store not configured")
        })?;

        let repo_root = repo
            .root_path_for_kind(self.supported_root_kind)
            .ok_or_else(|| {
                AgentError::new(
                    "UNSUPPORTED_REPO_ROOT",
                    format!(
                        "Repo {} root kind {} is unsupported by {} runtime",
                        repo.repo_id,
                        repo.root.kind(),
                        self.supported_root_kind.as_str()
                    ),
                )
            })?;

        let initial_entries = store.load(&repo.repo_id, repo_root).await;
        let classification_ignore_globs = self
            .config
            .as_ref()
            .map(|config| config.classification_excludes.to_ignore_globs())
            .unwrap_or_default();

        let watcher = RepoWatcher::start(RepoWatcherConfig {
            repo_id: repo.repo_id.clone(),
            root_path: repo_root.to_string(),
            docs_globs: repo.docs_globs.clone(),
            code_globs: repo.code_globs.clone(),
            ignore_globs: repo.ignore_globs.clone(),
            classification_ignore_globs,
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

    pub(crate) async fn ensure_repo_watcher_running(
        &mut self,
        repo: &RepoConfig,
        event_bus: &EventBus,
        logger: &Logger,
    ) -> Result<(), AgentError> {
        let repo_id = repo.repo_id.as_str();
        let watcher_finished = self
            .watchers
            .get(repo_id)
            .map(|watcher| watcher.is_task_finished())
            .unwrap_or(false);

        if watcher_finished {
            logger.warn(
                "Repo watcher task ended; restarting watcher",
                Some(json!({ "repoId": repo_id })),
            );
            if let Some(watcher) = self.watchers.remove(repo_id) {
                watcher.stop().await;
            }
        }

        if self.watchers.contains_key(repo_id) {
            return Ok(());
        }

        self.start_repo_watcher(repo, event_bus, logger).await
    }
}
