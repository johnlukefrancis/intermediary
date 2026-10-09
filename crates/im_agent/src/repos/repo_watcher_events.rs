// Path: crates/im_agent/src/repos/repo_watcher_events.rs
// Description: Event handling for repo watcher changes and rename mapping

use crate::logging::Logger;
use crate::protocol::{
    AgentEvent, FileChangeType, FileChangedEvent, FileEntry, FileKind, RepoTopologyChangedEvent,
};
use crate::repos::categorizer::Categorizer;
use crate::repos::delta::DeltaService;
use crate::repos::ignore_matcher::IgnoreMatcher;
use crate::repos::mru_index::MruIndex;
use crate::repos::recent_files_store::RecentFilesStore;
use crate::repos::repo_topology_change::event_affects_top_level_metadata;
use crate::repos::source_control_watch::SourceControlWatch;
use crate::server::EventBus;
use notify::event::ModifyKind;
use notify::{Event, EventKind};
use std::path::Path;
use tokio::sync::RwLock;

use crate::repos::repo_watcher_delta_marks::handle_rename_event;
pub(crate) use crate::repos::repo_watcher_delta_marks::{DeltaIntent, PendingRename};

/// State borrowed once by a watcher task to interpret every `notify` event.
pub(crate) struct EventContext<'a> {
    pub(crate) repo_id: &'a str,
    pub(crate) root_path: &'a Path,
    pub(crate) root_identity: &'a str,
    pub(crate) categorizer: &'a Categorizer,
    pub(crate) ignore_matcher: &'a IgnoreMatcher,
    pub(crate) mru: &'a RwLock<MruIndex>,
    pub(crate) recent_store: &'a RecentFilesStore,
    pub(crate) event_bus: &'a EventBus,
    pub(crate) logger: &'a Logger,
    pub(crate) source_control: &'a SourceControlWatch,
    pub(crate) delta: &'a DeltaService,
    /// The unpaired `RenameMode::From` half; see `repo_watcher_delta_marks`.
    pub(crate) pending_rename: PendingRename,
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

    /// The repo-relative slash path, or `None` for a path outside the root.
    pub(super) fn relative_of(&self, path: &Path) -> Option<String> {
        let relative = path.strip_prefix(self.root_path).ok()?;
        Some(
            relative
                .to_string_lossy()
                .replace(std::path::MAIN_SEPARATOR, "/"),
        )
    }

    /// What `apply_change` would publish for `path`: `None` when the watcher
    /// does not report it (outside the root, ignored, or `FileKind::Other`).
    fn classify(&self, path: &Path) -> Option<(String, FileKind)> {
        let Some(relative_str) = self.relative_of(path) else {
            self.logger.warn(
                "Skipping path outside repo root",
                Some(serde_json::json!({
                    "repoId": self.repo_id,
                    "path": path.to_string_lossy()
                })),
            );
            return None;
        };
        if self.ignore_matcher.should_ignore(&relative_str) {
            return None;
        }
        let kind = self.categorizer.categorize(&relative_str);
        if kind == FileKind::Other {
            return None;
        }
        Some((relative_str, kind))
    }

    /// Publishes `fileChanged` and optionally marks the delta queue;
    /// the return value lets rename arms mark one delta after both halves.
    pub(super) async fn apply_change(
        &self,
        path: &Path,
        change_type: FileChangeType,
        intent: DeltaIntent,
    ) -> Option<(String, FileKind)> {
        let (relative_str, kind) = self.classify(path)?;

        let mtime = match change_type {
            FileChangeType::Unlink => chrono::Utc::now(),
            _ => match tokio::fs::metadata(path).await {
                Ok(metadata) => {
                    if metadata.is_dir() {
                        return None;
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
                self.root_identity.to_string(),
                entries,
            )
            .await;

        let event_payload = FileChangedEvent::new(
            self.repo_id.to_string(),
            self.root_identity.to_string(),
            relative_str.clone(),
            kind,
            change_type,
            mtime.to_rfc3339(),
            updated_entry.activity,
        );
        self.event_bus
            .broadcast_event(AgentEvent::FileChanged(event_payload));

        if intent == DeltaIntent::Note {
            self.note_delta(relative_str.clone(), path, kind, change_type);
        }
        Some((relative_str, kind))
    }
}

pub(crate) async fn handle_event(context: &EventContext<'_>, event: Event) {
    context.maybe_broadcast_topology_changed(&event).await;

    // Git status also moves for ignored paths, other kinds and git metadata;
    // note them before the file event filters and rename branch.
    context.source_control.note_event(&event);

    // External git-dir events only feed the source control detector;
    // paths outside the root produce no file event or log.
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
        context
            .apply_change(path, change_type, DeltaIntent::Note)
            .await;
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

fn map_code(code: i32) -> String {
    if code == libc::ENOSPC {
        return "ENOSPC".to_string();
    }
    if code == libc::EMFILE {
        return "EMFILE".to_string();
    }
    code.to_string()
}
