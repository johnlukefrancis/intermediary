// Path: crates/im_agent/src/repos/recent_files_store.rs
// Description: Persist recent files with debounced atomic writes

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::sync::Mutex;
use tokio::time::sleep;

use crate::logging::Logger;
use crate::protocol::FileEntry;

const PERSIST_DEBOUNCE_MS: u64 = 500;
const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedRecentFiles {
    version: u32,
    repo_id: String,
    repo_root: String,
    updated_at_iso: String,
    entries: Vec<FileEntry>,
}

struct PendingWrite {
    repo_root: String,
    entries: Vec<FileEntry>,
    handle: Option<tokio::task::JoinHandle<()>>,
}

#[derive(Clone)]
pub struct RecentFilesStore {
    state_dir: PathBuf,
    logger: Logger,
    pending: std::sync::Arc<Mutex<HashMap<String, PendingWrite>>>,
}

impl RecentFilesStore {
    pub fn new(state_dir: PathBuf, logger: Logger) -> Self {
        Self {
            state_dir,
            logger,
            pending: std::sync::Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn load(&self, repo_id: &str, repo_root: &str) -> Vec<FileEntry> {
        let file_path = self.file_path(repo_id);
        if fs::metadata(&file_path).await.is_err() {
            return Vec::new();
        }

        let content = match fs::read_to_string(&file_path).await {
            Ok(content) => content,
            Err(err) => {
                self.logger.warn(
                    "Failed to read recent files",
                    Some(serde_json::json!({"repoId": repo_id, "error": err.to_string()})),
                );
                return Vec::new();
            }
        };

        let data: PersistedRecentFiles = match serde_json::from_str(&content) {
            Ok(data) => data,
            Err(_) => {
                self.logger.warn(
                    "Corrupt recent files JSON",
                    Some(serde_json::json!({"repoId": repo_id})),
                );
                return Vec::new();
            }
        };

        if data.version != SCHEMA_VERSION {
            self.logger.warn(
                "Unsupported recent files schema",
                Some(serde_json::json!({"repoId": repo_id, "found": data.version, "expected": SCHEMA_VERSION})),
            );
            return Vec::new();
        }

        if data.repo_root != repo_root {
            self.logger.info(
                "Repo root changed for recent files",
                Some(serde_json::json!({"repoId": repo_id, "stored": data.repo_root, "current": repo_root})),
            );
        }

        data.entries
    }

    pub async fn schedule_save(&self, repo_id: String, repo_root: String, entries: Vec<FileEntry>) {
        let store = self.clone();
        let repo_id_clone = repo_id.clone();

        let mut pending = self.pending.lock().await;
        if let Some(existing) = pending.get_mut(&repo_id) {
            if let Some(handle) = existing.handle.take() {
                handle.abort();
            }
            existing.repo_root = repo_root;
            existing.entries = entries;
        } else {
            pending.insert(
                repo_id.clone(),
                PendingWrite {
                    repo_root,
                    entries,
                    handle: None,
                },
            );
        }

        let handle = tokio::spawn(async move {
            sleep(Duration::from_millis(PERSIST_DEBOUNCE_MS)).await;
            store.write_pending(&repo_id_clone).await;
        });

        if let Some(existing) = pending.get_mut(&repo_id) {
            existing.handle = Some(handle);
        }
    }

    pub async fn flush_repo(&self, repo_id: &str) {
        let pending_entry = { self.pending.lock().await.remove(repo_id) };

        if let Some(mut entry) = pending_entry {
            if let Some(handle) = entry.handle.take() {
                handle.abort();
            }
            self.write_now(repo_id.to_string(), entry.repo_root, entry.entries)
                .await;
        }
    }

    pub async fn flush_all(&self) {
        let entries = {
            let mut pending = self.pending.lock().await;
            pending
                .drain()
                .map(|(key, value)| (key, value))
                .collect::<Vec<_>>()
        };

        for (repo_id, entry) in entries {
            let mut entry = entry;
            if let Some(handle) = entry.handle.take() {
                handle.abort();
            }
            self.write_now(repo_id, entry.repo_root, entry.entries)
                .await;
        }
    }

    async fn write_pending(&self, repo_id: &str) {
        let pending_entry = { self.pending.lock().await.remove(repo_id) };
        if let Some(entry) = pending_entry {
            self.write_now(repo_id.to_string(), entry.repo_root, entry.entries)
                .await;
        }
    }

    async fn write_now(&self, repo_id: String, repo_root: String, entries: Vec<FileEntry>) {
        let file_path = self.file_path(&repo_id);
        let dir = file_path.parent().unwrap_or_else(|| Path::new("."));

        let payload = PersistedRecentFiles {
            version: SCHEMA_VERSION,
            repo_id,
            repo_root,
            updated_at_iso: chrono::Utc::now().to_rfc3339(),
            entries,
        };

        if let Err(err) = fs::create_dir_all(dir).await {
            self.logger.error(
                "Failed to create recent files dir",
                Some(serde_json::json!({"error": err.to_string()})),
            );
            return;
        }

        let temp_path = file_path.with_extension(format!(
            "tmp-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        let content = match serde_json::to_string_pretty(&payload) {
            Ok(content) => content,
            Err(err) => {
                self.logger.error(
                    "Failed to serialize recent files",
                    Some(serde_json::json!({"error": err.to_string()})),
                );
                return;
            }
        };

        if let Err(err) = fs::write(&temp_path, content).await {
            self.logger.error(
                "Failed to write recent files",
                Some(serde_json::json!({"error": err.to_string()})),
            );
            let _ = fs::remove_file(&temp_path).await;
            return;
        }

        if let Err(err) = fs::rename(&temp_path, &file_path).await {
            self.logger.error(
                "Failed to finalize recent files",
                Some(serde_json::json!({"error": err.to_string()})),
            );
            let _ = fs::remove_file(&temp_path).await;
        }
    }

    fn file_path(&self, repo_id: &str) -> PathBuf {
        self.state_dir
            .join("recent_files")
            .join(format!("{repo_id}.json"))
    }
}
