// Path: crates/im_agent/src/repos/repo_watcher.rs
// Description: Notify-based repo watcher with MRU and event emission

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, watch, Mutex, RwLock};

use crate::error::AgentError;
use crate::logging::Logger;
use crate::protocol::{AgentEvent, FileEntry, SnapshotEvent};
use crate::repos::categorizer::Categorizer;
use crate::repos::ignore_matcher::IgnoreMatcher;
use crate::repos::mru_index::MruIndex;
use crate::repos::recent_files_store::RecentFilesStore;
use crate::repos::repo_watcher_events::{handle_event, raw_os_code, EventContext};
use crate::repos::source_control_watch::{
    load_tracked_paths, resolve_external_watches, SourceControlChangeDetector, SourceControlWatch,
    TrackedPathSet,
};
use crate::repos::watcher_error::build_watcher_error_event;
use crate::server::EventBus;

pub struct RepoWatcherConfig {
    pub repo_id: String,
    pub root_path: String,
    pub docs_globs: Vec<String>,
    pub code_globs: Vec<String>,
    pub ignore_globs: Vec<String>,
    pub classification_ignore_globs: Vec<String>,
    pub mru_capacity: usize,
    pub recent_store: RecentFilesStore,
    pub logger: Logger,
    pub event_bus: EventBus,
}

pub struct RepoWatcher {
    repo_id: String,
    root_path: PathBuf,
    mru: Arc<RwLock<MruIndex>>,
    recent_store: RecentFilesStore,
    logger: Logger,
    event_bus: EventBus,
    watcher: Mutex<Option<RecommendedWatcher>>,
    extra_watch_paths: Vec<PathBuf>,
    stop_tx: watch::Sender<bool>,
    task: tokio::task::JoinHandle<()>,
}

impl RepoWatcher {
    pub async fn start(config: RepoWatcherConfig) -> Result<Self, AgentError> {
        let categorizer = Categorizer::new(&config.docs_globs, &config.code_globs)?;
        let mut combined_ignore_globs = config.ignore_globs.clone();
        combined_ignore_globs.extend(config.classification_ignore_globs.clone());
        let ignore_matcher = IgnoreMatcher::new(&combined_ignore_globs)?;

        let mut mru = MruIndex::new(config.mru_capacity).map_err(AgentError::internal)?;
        let initial_entries = config
            .recent_store
            .load(
                &config.repo_id,
                &config.root_path,
                Some(&categorizer),
                Some(&ignore_matcher),
            )
            .await;
        let initial_entries = filter_initial_entries(initial_entries, &ignore_matcher);
        if !initial_entries.is_empty() {
            mru.load_from(initial_entries);
        }

        // A linked worktree keeps its git dir outside the root, so `git status`
        // can move without a single event under the watched tree.
        let root_path = PathBuf::from(&config.root_path);
        let external_watches = resolve_external_watches(&root_path, &config.logger).await;

        // Loaded once here so the detector's tracked-path override is live
        // from the watcher's first event; a `.git/index` change later
        // refreshes it in place (`SourceControlWatch::note_event`).
        let tracked = TrackedPathSet::empty();
        match load_tracked_paths(&root_path).await {
            Ok(paths) => tracked.store(paths),
            Err(reason) => {
                config.logger.warn(
                    "Source control watch has no tracked-path signal",
                    Some(serde_json::json!({
                        "rootPath": root_path.to_string_lossy(),
                        "reason": reason,
                    })),
                );
            }
        }

        let detector = SourceControlChangeDetector::new(
            &root_path,
            external_watches.detector_dirs,
            &config.ignore_globs,
            tracked.clone(),
        )?;
        let extra_watch_paths: Vec<PathBuf> = external_watches
            .watch_paths
            .iter()
            .map(|(path, _)| path.clone())
            .collect();

        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<Result<Event, notify::Error>>();
        let watch_root = config.root_path.clone();
        let extra_watches = external_watches.watch_paths;
        let watch_logger = config.logger.clone();
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
                    watch_logger.warn(
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

        let (stop_tx, mut stop_rx) = watch::channel(false);

        let repo_id = config.repo_id.clone();
        let logger = config.logger.clone();
        let event_bus = config.event_bus.clone();
        let recent_store = config.recent_store.clone();

        let mru_lock = Arc::new(RwLock::new(mru));
        let mru_clone = Arc::clone(&mru_lock);
        let task = tokio::spawn(async move {
            let source_control = SourceControlWatch::new(
                repo_id.clone(),
                event_bus.clone(),
                detector,
                tracked,
                root_path.clone(),
                logger.clone(),
            );
            let context = EventContext {
                repo_id: &repo_id,
                root_path: &root_path,
                categorizer: &categorizer,
                ignore_matcher: &ignore_matcher,
                mru: &mru_clone,
                recent_store: &recent_store,
                event_bus: &event_bus,
                logger: &logger,
                source_control: &source_control,
            };

            // Armed only while the coalescer owes a trailing event, so an idle
            // watcher holds no timer and never spins.
            let flush_timer = tokio::time::sleep(Duration::ZERO);
            tokio::pin!(flush_timer);
            let mut source_control_pending = false;

            loop {
                tokio::select! {
                    _ = stop_rx.changed() => {
                        break;
                    }
                    _ = &mut flush_timer, if source_control_pending => {
                        source_control_pending = false;
                        source_control.flush();
                    }
                    message = event_rx.recv() => {
                        let message = match message {
                            Some(message) => message,
                            None => break,
                        };

                        match message {
                            Ok(event) => {
                                handle_event(&context, event).await;
                                if let Some(deadline) = source_control.pending_deadline() {
                                    flush_timer
                                        .as_mut()
                                        .reset(tokio::time::Instant::from_std(deadline));
                                    source_control_pending = true;
                                }
                            }
                            Err(err) => {
                                let raw_code = raw_os_code(&err);
                                let raw_message = err.to_string();
                                let event = build_watcher_error_event(&repo_id, raw_message, raw_code);
                                event_bus.broadcast_event(AgentEvent::Error(event));
                            }
                        }
                    }
                }
            }

            recent_store.flush_repo(&repo_id).await;
        });

        Ok(Self {
            repo_id: config.repo_id,
            root_path: PathBuf::from(config.root_path),
            mru: mru_lock,
            recent_store: config.recent_store,
            logger: config.logger,
            event_bus: config.event_bus,
            watcher: Mutex::new(Some(watcher)),
            extra_watch_paths,
            stop_tx,
            task,
        })
    }

    pub fn repo_id(&self) -> &str {
        &self.repo_id
    }

    pub async fn stop(&self) {
        let _ = self.stop_tx.send(true);
        if let Some(mut watcher) = self.watcher.lock().await.take() {
            let root_path = self.root_path.clone();
            let extra_watch_paths = self.extra_watch_paths.clone();
            let _ = tokio::task::spawn_blocking(move || {
                let _ = watcher.unwatch(&root_path);
                for path in &extra_watch_paths {
                    let _ = watcher.unwatch(path);
                }
            })
            .await;
        }
        self.task.abort();
        self.recent_store.flush_repo(&self.repo_id).await;
        self.logger.info(
            "Repo watcher stopped",
            Some(serde_json::json!({"repoId": self.repo_id})),
        );
    }

    pub async fn recent_entries(&self) -> Vec<FileEntry> {
        let mru = self.mru.read().await;
        mru.entries()
    }

    pub fn is_task_finished(&self) -> bool {
        self.task.is_finished()
    }

    pub fn broadcast_snapshot(&self) {
        let repo_id = self.repo_id.clone();
        let event_bus = self.event_bus.clone();
        let mru = Arc::clone(&self.mru);
        tokio::spawn(async move {
            let entries = { mru.read().await.entries() };
            let snapshot = SnapshotEvent::new(repo_id, entries);
            event_bus.broadcast_event(AgentEvent::Snapshot(snapshot));
        });
    }
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
