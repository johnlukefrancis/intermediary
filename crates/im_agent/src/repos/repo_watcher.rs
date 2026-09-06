// Path: crates/im_agent/src/repos/repo_watcher.rs
// Description: Notify-based repo watcher with MRU, delta pipeline, and event emission

use notify::{RecommendedWatcher, Watcher};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{watch, Mutex, RwLock};

use crate::error::AgentError;
use crate::logging::Logger;
use crate::protocol::{AgentEvent, FileEntry, SnapshotEvent};
use crate::repos::categorizer::Categorizer;
use crate::repos::delta::{DeltaLimits, DeltaService};
use crate::repos::ignore_matcher::IgnoreMatcher;
use crate::repos::mru_index::MruIndex;
use crate::repos::recent_files_store::RecentFilesStore;
use crate::repos::repo_watcher_events::{handle_event, raw_os_code, EventContext, PendingRename};
use crate::repos::repo_watcher_startup::{
    create_watcher, filter_initial_entries, load_tracked_set,
};
use crate::repos::source_control_watch::{
    resolve_external_watches, SourceControlChangeDetector, SourceControlWatch,
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
    pub delta_limits: DeltaLimits,
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
    delta: Arc<DeltaService>,
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

        let root_path = PathBuf::from(&config.root_path);
        let external_watches = resolve_external_watches(&root_path, &config.logger).await;
        let tracked = load_tracked_set(&root_path, &config.logger).await;

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

        let (watcher, mut event_rx) = create_watcher(
            config.root_path.clone(),
            external_watches.watch_paths,
            config.logger.clone(),
        )
        .await?;

        let (stop_tx, mut stop_rx) = watch::channel(false);

        let repo_id = config.repo_id.clone();
        let logger = config.logger.clone();
        let event_bus = config.event_bus.clone();
        let recent_store = config.recent_store.clone();

        // Built after the tracked set so the first delta can stamp `tracked`;
        // the worker starts here and is stopped in `stop` before the task.
        let delta = Arc::new(DeltaService::new(
            repo_id.clone(),
            root_path.clone(),
            event_bus.clone(),
            logger.clone(),
            tracked.clone(),
            config.delta_limits.clone(),
        ));
        let delta_task = Arc::clone(&delta);

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
                delta: &delta_task,
                pending_rename: PendingRename::new(),
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
            delta,
            stop_tx,
            task,
        })
    }

    pub fn repo_id(&self) -> &str {
        &self.repo_id
    }

    /// Stop order: task loop, unwatch, delta worker, task abort, recents flush.
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
        self.delta.stop().await;
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
