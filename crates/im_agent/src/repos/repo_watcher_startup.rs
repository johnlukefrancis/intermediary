// Path: crates/im_agent/src/repos/repo_watcher_startup.rs
// Description: Repo watcher startup - notify watcher creation, external git watches, initial tracked-path load

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

use crate::error::AgentError;
use crate::logging::Logger;
use crate::protocol::FileEntry;
use crate::repos::ignore_matcher::IgnoreMatcher;
use crate::repos::source_control_watch::{load_tracked_paths, TrackedPathSet};

pub(super) type WatcherMessage = Result<Event, notify::Error>;

/// Loaded once so the detector's tracked-path override is live from the
/// watcher's first event; a `.git/index` change later refreshes it in place
/// (`SourceControlWatch::note_event`). A failed load leaves the set empty and
/// logs once.
pub(super) async fn load_tracked_set(root_path: &Path, logger: &Logger) -> TrackedPathSet {
    let tracked = TrackedPathSet::empty();
    match load_tracked_paths(root_path).await {
        Ok(paths) => tracked.store(paths),
        Err(reason) => {
            logger.warn(
                "Source control watch has no tracked-path signal",
                Some(serde_json::json!({
                    "rootPath": root_path.to_string_lossy(),
                    "reason": reason,
                })),
            );
        }
    }
    tracked
}

/// Creates the `notify` watcher on the blocking pool, watching the repo root
/// recursively plus any external git dirs (a linked worktree keeps its git dir
/// outside the root). Events arrive on the returned channel.
pub(super) async fn create_watcher(
    watch_root: String,
    extra_watches: Vec<(PathBuf, RecursiveMode)>,
    logger: Logger,
) -> Result<(RecommendedWatcher, mpsc::UnboundedReceiver<WatcherMessage>), AgentError> {
    let (event_tx, event_rx) = mpsc::unbounded_channel::<WatcherMessage>();
    let watcher = tokio::task::spawn_blocking(move || {
        let mut watcher = notify::recommended_watcher(move |res| {
            let _ = event_tx.send(res);
        })
        .map_err(|err| AgentError::internal(format!("Failed to create watcher: {err}")))?;

        watcher
            .watch(Path::new(&watch_root), RecursiveMode::Recursive)
            .map_err(|err| AgentError::internal(format!("Failed to watch repo: {err}")))?;
        for (path, mode) in &extra_watches {
            if let Err(err) = watcher.watch(path, *mode) {
                logger.warn(
                    "Failed to watch external git dir",
                    Some(serde_json::json!({
                        "path": path.to_string_lossy(),
                        "error": err.to_string(),
                    })),
                );
            }
        }
        Ok::<RecommendedWatcher, AgentError>(watcher)
    })
    .await
    .map_err(|err| AgentError::internal(format!("Watcher startup task failed: {err}")))??;
    Ok((watcher, event_rx))
}

pub(super) fn filter_initial_entries(
    entries: Vec<FileEntry>,
    ignore_matcher: &IgnoreMatcher,
) -> Vec<FileEntry> {
    entries
        .into_iter()
        .filter(|entry| {
            let normalized_path = entry.path.replace('\\', "/");
            !ignore_matcher.should_ignore(&normalized_path)
        })
        .collect()
}
