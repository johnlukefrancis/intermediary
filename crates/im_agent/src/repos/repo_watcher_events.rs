// Path: crates/im_agent/src/repos/repo_watcher_events.rs
// Description: Event handling for repo watcher changes and rename mapping

use crate::logging::Logger;
use crate::protocol::{
    AgentEvent, FileChangeType, FileChangedEvent, FileEntry, FileKind, RepoTopologyChangedEvent,
};
use crate::repos::categorizer::Categorizer;
use crate::repos::ignore_matcher::IgnoreMatcher;
use crate::repos::mru_index::MruIndex;
use crate::repos::recent_files_store::RecentFilesStore;
use crate::repos::repo_topology_change::event_affects_top_level_metadata;
use crate::repos::source_control_watch::SourceControlWatch;
use crate::server::EventBus;
use notify::event::{ModifyKind, RenameMode};
use notify::{Event, EventKind};
use std::path::{Path, PathBuf};
use tokio::sync::RwLock;

/// Everything one watcher task needs to interpret a raw `notify` event. Built
/// once per watcher and reused for every event, so new signals ride here
/// instead of lengthening `handle_event`.
pub(crate) struct EventContext<'a> {
    pub(crate) repo_id: &'a str,
    pub(crate) root_path: &'a Path,
    pub(crate) categorizer: &'a Categorizer,
    pub(crate) ignore_matcher: &'a IgnoreMatcher,
    pub(crate) mru: &'a RwLock<MruIndex>,
    pub(crate) recent_store: &'a RecentFilesStore,
    pub(crate) event_bus: &'a EventBus,
    pub(crate) logger: &'a Logger,
    pub(crate) source_control: &'a SourceControlWatch,
}

impl<'a> EventContext<'a> {
    async fn maybe_broadcast_topology_changed(&self, event: &Event) {
        if event_affects_top_level_metadata(self.root_path, event).await {
            self.event_bus
                .broadcast_event(AgentEvent::RepoTopologyChanged(
                    RepoTopologyChangedEvent::new(self.repo_id.to_string()),
                ));
        }
    }

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
            activity: None,
        };

        let mut updated_entry = entry.clone();
        let entries = {
            let mut guard = self.mru.write().await;
            if change_type == FileChangeType::Unlink {
                guard.remove(&relative_str);
            } else {
                updated_entry = guard.upsert(entry);
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
            updated_entry.activity,
        );
        self.event_bus
            .broadcast_event(AgentEvent::FileChanged(event_payload));
    }
}

pub(crate) async fn handle_event(context: &EventContext<'_>, event: Event) {
    context.maybe_broadcast_topology_changed(&event).await;

    // Before the rename branch and before `map_event_kind` drops unhandled
    // kinds: `git status` moves for paths `apply_change` never sees (ignored
    // globs, `FileKind::Other`, git metadata).
    context.source_control.note_event(&event);

    // An event from an external git-dir watch (a linked worktree's git dir or
    // common dir) has no path under the root; the detector above was its only
    // consumer, so it yields no file event and no log.
    if !event
        .paths
        .iter()
        .any(|path| path.starts_with(context.root_path))
    {
        return;
    }

    if let EventKind::Modify(ModifyKind::Name(mode)) = event.kind {
        handle_rename_event(context, mode, &event.paths).await;
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
