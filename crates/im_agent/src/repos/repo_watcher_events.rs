// Path: crates/im_agent/src/repos/repo_watcher_events.rs
// Description: Event handling for repo watcher changes and rename mapping

use crate::logging::Logger;
use crate::protocol::{AgentEvent, FileChangeType, FileChangedEvent, FileEntry, FileKind};
use crate::repos::categorizer::Categorizer;
use crate::repos::ignore_matcher::IgnoreMatcher;
use crate::repos::mru_index::MruIndex;
use crate::repos::recent_files_store::RecentFilesStore;
use crate::server::EventBus;
use notify::event::{ModifyKind, RenameMode};
use notify::{Event, EventKind};
use std::path::{Path, PathBuf};
use tokio::sync::RwLock;

struct EventContext<'a> {
    repo_id: &'a str,
    root_path: &'a Path,
    categorizer: &'a Categorizer,
    ignore_matcher: &'a IgnoreMatcher,
    mru: &'a RwLock<MruIndex>,
    recent_store: &'a RecentFilesStore,
    event_bus: &'a EventBus,
    logger: &'a Logger,
}

impl<'a> EventContext<'a> {
    async fn apply_change(&self, path: &Path, change_type: FileChangeType) {
        let relative_path = match path.strip_prefix(self.root_path) {
            Ok(relative) => relative,
            Err(_) => {
                self.logger.warn(
                    "Skipping path outside repo root",
                    Some(serde_json::json!({
                        "repoId": self.repo_id,
                        "path": path.to_string_lossy()
                    })),
                );
                return;
            }
        };

        let relative_str = relative_path
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/");

        if self.ignore_matcher.should_ignore(&relative_str) {
            return;
        }

        let kind = self.categorizer.categorize(&relative_str);
        if kind == FileKind::Other {
            return;
        }

        let mtime = match change_type {
            FileChangeType::Unlink => chrono::Utc::now(),
            _ => match tokio::fs::metadata(path).await {
                Ok(metadata) => {
                    if metadata.is_dir() {
                        return;
                    }
                    match metadata.modified() {
                        Ok(modified) => chrono::DateTime::<chrono::Utc>::from(modified),
                        Err(_) => chrono::Utc::now(),
                    }
                }
                Err(_) => chrono::Utc::now(),
            },
        };

        let entry = FileEntry {
            path: relative_str.clone(),
            kind,
            change_type,
            mtime: mtime.to_rfc3339(),
            size_bytes: None,
        };

        let entries = {
            let mut guard = self.mru.write().await;
            if change_type == FileChangeType::Unlink {
                guard.remove(&relative_str);
            } else {
                guard.upsert(entry.clone());
            }
            guard.entries()
        };
        self.recent_store
            .schedule_save(
                self.repo_id.to_string(),
                self.root_path.to_string_lossy().to_string(),
                entries,
            )
            .await;

        let event_payload = FileChangedEvent::new(
            self.repo_id.to_string(),
            relative_str,
            kind,
            change_type,
            mtime.to_rfc3339(),
        );
        self.event_bus
            .broadcast_event(AgentEvent::FileChanged(event_payload));
    }
}

pub(crate) async fn handle_event(
    repo_id: &str,
    root_path: &Path,
    event: Event,
    categorizer: &Categorizer,
    ignore_matcher: &IgnoreMatcher,
    mru: &RwLock<MruIndex>,
    recent_store: &RecentFilesStore,
    event_bus: &EventBus,
    logger: &Logger,
) {
    let context = EventContext {
        repo_id,
        root_path,
        categorizer,
        ignore_matcher,
        mru,
        recent_store,
        event_bus,
        logger,
    };

    if let EventKind::Modify(ModifyKind::Name(mode)) = event.kind {
        handle_rename_event(&context, mode, &event.paths).await;
        return;
    }

    let change_type = match map_event_kind(&event.kind) {
        Some(change_type) => change_type,
        None => return,
    };

    for path in &event.paths {
        context.apply_change(path, change_type).await;
    }
}

pub(crate) fn raw_os_code(err: &notify::Error) -> Option<String> {
    match err.kind {
        notify::ErrorKind::Io(ref io_err) => io_err.raw_os_error().map(map_code),
        _ => None,
    }
}

fn map_event_kind(kind: &EventKind) -> Option<FileChangeType> {
    match kind {
        EventKind::Create(_) => Some(FileChangeType::Add),
        EventKind::Modify(_) => Some(FileChangeType::Change),
        EventKind::Remove(_) => Some(FileChangeType::Unlink),
        _ => None,
    }
}

async fn handle_rename_event(context: &EventContext<'_>, mode: RenameMode, paths: &[PathBuf]) {
    match mode {
        RenameMode::Both => {
            if let Some(from_path) = paths.get(0) {
                context
                    .apply_change(from_path, FileChangeType::Unlink)
                    .await;
            }
            if let Some(to_path) = paths.get(1) {
                context.apply_change(to_path, FileChangeType::Add).await;
            }
        }
        RenameMode::From => {
            if let Some(from_path) = paths.get(0) {
                context
                    .apply_change(from_path, FileChangeType::Unlink)
                    .await;
            }
        }
        RenameMode::To => {
            if let Some(to_path) = paths.get(0) {
                context.apply_change(to_path, FileChangeType::Add).await;
            }
        }
        RenameMode::Any | RenameMode::Other => {
            if paths.len() >= 2 {
                if let Some(from_path) = paths.get(0) {
                    context
                        .apply_change(from_path, FileChangeType::Unlink)
                        .await;
                }
                if let Some(to_path) = paths.get(1) {
                    context.apply_change(to_path, FileChangeType::Add).await;
                }
            } else if let Some(path) = paths.get(0) {
                let change_type = infer_rename_change_type(path).await;
                context.apply_change(path, change_type).await;
            }
        }
    }
}

async fn infer_rename_change_type(path: &Path) -> FileChangeType {
    match tokio::fs::metadata(path).await {
        Ok(_) => FileChangeType::Add,
        Err(_) => FileChangeType::Unlink,
    }
}

fn map_code(code: i32) -> String {
    if code == libc::ENOSPC {
        return "ENOSPC".to_string();
    }
    if code == libc::EMFILE {
        return "EMFILE".to_string();
    }
    code.to_string()
}
