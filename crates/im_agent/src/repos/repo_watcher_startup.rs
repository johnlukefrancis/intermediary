// Path: crates/im_agent/src/repos/repo_watcher_startup.rs
// Description: Repo watcher startup - notify watcher creation, background watch registration, initial tracked-path load

use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::mpsc;

use crate::error::AgentError;
use crate::logging::Logger;
use crate::protocol::{AgentEvent, FileEntry};
use crate::repos::git_ignore::GitIgnoreFilter;
use crate::repos::ignore_matcher::IgnoreMatcher;
use crate::repos::repo_watcher_events::raw_os_code;
use crate::repos::source_control_watch::{load_tracked_paths, TrackedPathSet};
use crate::repos::watcher_error::build_watcher_error_event;
use crate::server::EventBus;

pub(super) type WatcherMessage = Result<Event, notify::Error>;

/// Loaded before the first event so the detector's tracked-path override is live;
/// `.git/index` changes refresh it in place and a failed load logs once.
pub(super) async fn load_tracked_set(root_path: &Path, tracked: &TrackedPathSet, logger: &Logger) {
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
}

/// Holds the watcher once registration hands it back; `stopped` makes a still-registering
/// task drop it, and `root_failed` marks a watcher that `watch_repo`/reconcile must restart.
#[derive(Default)]
pub(super) struct WatcherSlot {
    watcher: Option<RecommendedWatcher>,
    stopped: bool,
    root_failed: bool,
}

pub(super) type SharedWatcherSlot = Arc<StdMutex<WatcherSlot>>;

pub(super) struct WatchRegistration {
    pub repo_id: String,
    pub root_identity: String,
    pub watch_root: PathBuf,
    pub extra_watches: Vec<(PathBuf, RecursiveMode)>,
    pub event_bus: EventBus,
    pub logger: Logger,
}

/// Symlinks are never followed so a link out of the root (`work -> /mnt/j/...`)
/// registers no watches; events arrive on the returned channel.
pub(super) fn create_watcher(
) -> Result<(RecommendedWatcher, mpsc::UnboundedReceiver<WatcherMessage>), AgentError> {
    let (event_tx, event_rx) = mpsc::unbounded_channel::<WatcherMessage>();
    let config = Config::default().with_follow_symlinks(false);
    let watcher = RecommendedWatcher::new(
        move |res| {
            let _ = event_tx.send(res);
        },
        config,
    )
    .map_err(|err| AgentError::internal(format!("Failed to create watcher: {err}")))?;
    Ok((watcher, event_rx))
}

/// Recursive registration walks the whole tree on inotify, so it runs on the
/// blocking pool after the watcher exists; root failure is a watcher error event.
pub(super) fn spawn_watch_registration(
    mut watcher: RecommendedWatcher,
    slot: SharedWatcherSlot,
    registration: WatchRegistration,
) {
    tokio::task::spawn_blocking(move || {
        let root_result = watcher.watch(&registration.watch_root, RecursiveMode::Recursive);
        if let Err(err) = &root_result {
            report_root_watch_failure(&registration, err);
        }
        for (path, mode) in &registration.extra_watches {
            if let Err(err) = watcher.watch(path, *mode) {
                registration.logger.warn(
                    "Failed to watch external git dir",
                    Some(serde_json::json!({
                        "path": path.to_string_lossy(),
                        "error": err.to_string(),
                    })),
                );
            }
        }
        let mut guard = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if !guard.stopped {
            guard.watcher = Some(watcher);
            guard.root_failed = root_result.is_err();
            return;
        }
        drop(guard);
        let extras: Vec<&Path> = registration
            .extra_watches
            .iter()
            .map(|(path, _)| path.as_path())
            .collect();
        unwatch_all(&mut watcher, &registration.watch_root, &extras);
    });
}

/// Marks the slot stopped and unwatches a registered watcher; a registration
/// still in flight sees `stopped` and releases its own watcher.
pub(super) async fn stop_watcher_slot(
    slot: &SharedWatcherSlot,
    root_path: PathBuf,
    extra_watch_paths: Vec<PathBuf>,
) {
    let watcher = {
        let mut guard = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.stopped = true;
        guard.watcher.take()
    };
    let Some(mut watcher) = watcher else {
        return;
    };
    let _ = tokio::task::spawn_blocking(move || {
        let extras: Vec<&Path> = extra_watch_paths.iter().map(PathBuf::as_path).collect();
        unwatch_all(&mut watcher, &root_path, &extras);
    })
    .await;
}

/// The root watch failed, so this watcher sees nothing and must be replaced.
pub(super) fn root_registration_failed(slot: &SharedWatcherSlot) -> bool {
    let guard = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    guard.root_failed
}

/// True once background registration has handed the watcher back, failed root watch included.
#[cfg(test)]
pub(super) fn registration_finished(slot: &SharedWatcherSlot) -> bool {
    let guard = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    guard.watcher.is_some()
}

fn unwatch_all(watcher: &mut RecommendedWatcher, root: &Path, extras: &[&Path]) {
    let _ = watcher.unwatch(root);
    for path in extras {
        let _ = watcher.unwatch(path);
    }
}

fn report_root_watch_failure(registration: &WatchRegistration, err: &notify::Error) {
    registration.logger.error(
        "Failed to register repo watches",
        Some(serde_json::json!({
            "repoId": registration.repo_id,
            "rootPath": registration.root_identity,
            "error": err.to_string(),
        })),
    );
    let event = build_watcher_error_event(
        &registration.repo_id,
        &registration.root_identity,
        format!("Failed to watch repo: {err}"),
        raw_os_code(err),
    );
    registration
        .event_bus
        .broadcast_event(AgentEvent::Error(event));
}

pub(super) fn filter_initial_entries(
    entries: Vec<FileEntry>,
    ignore_matcher: &IgnoreMatcher,
    git_ignore: &GitIgnoreFilter,
) -> Vec<FileEntry> {
    entries
        .into_iter()
        .filter(|entry| {
            let normalized_path = entry.path.replace('\\', "/");
            !ignore_matcher.should_ignore(&normalized_path)
                && !git_ignore.is_suppressed(&normalized_path)
        })
        .collect()
}

#[cfg(test)]
#[path = "repo_watcher_startup_tests.rs"]
mod tests;
