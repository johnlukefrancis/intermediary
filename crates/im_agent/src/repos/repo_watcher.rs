// Path: crates/im_agent/src/repos/repo_watcher.rs
// Description: Notify-based repo watcher with MRU and event emission

use std::path::{Path, PathBuf};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock, watch};

use crate::error::AgentError;
use crate::logging::Logger;
use crate::protocol::{AgentEvent, FileEntry, SnapshotEvent};
use crate::repos::categorizer::Categorizer;
use crate::repos::ignore_matcher::IgnoreMatcher;
use crate::repos::mru_index::MruIndex;
use crate::repos::recent_files_store::RecentFilesStore;
use crate::repos::watcher_error::build_watcher_error_event;
use crate::repos::repo_watcher_events::{handle_event, raw_os_code};
use crate::server::EventBus;

pub struct RepoWatcherConfig {
    pub repo_id: String,
    pub root_path: String,
    pub docs_globs: Vec<String>,
    pub code_globs: Vec<String>,
    pub ignore_globs: Vec<String>,
    pub mru_capacity: usize,
    pub initial_entries: Vec<FileEntry>,
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
    stop_tx: watch::Sender<bool>,
    task: tokio::task::JoinHandle<()>,
}

impl RepoWatcher {
    pub async fn start(config: RepoWatcherConfig) -> Result<Self, AgentError> {
        let mut mru = MruIndex::new(config.mru_capacity)
            .map_err(|err| AgentError::internal(err))?;
        if !config.initial_entries.is_empty() {
            mru.load_from(config.initial_entries);
        }

        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<Result<Event, notify::Error>>();

        let mut watcher = notify::recommended_watcher(move |res| {
            let _ = event_tx.send(res);
        })
        .map_err(|err| AgentError::internal(format!("Failed to create watcher: {err}")))?;

        watcher
            .watch(Path::new(&config.root_path), RecursiveMode::Recursive)
            .map_err(|err| AgentError::internal(format!("Failed to watch repo: {err}")))?;

        let (stop_tx, mut stop_rx) = watch::channel(false);

        let repo_id = config.repo_id.clone();
        let root_path = PathBuf::from(&config.root_path);
        let logger = config.logger.clone();
        let event_bus = config.event_bus.clone();
        let recent_store = config.recent_store.clone();

        let categorizer = Categorizer::new(&config.docs_globs, &config.code_globs)?;
        let ignore_matcher = IgnoreMatcher::new(&config.ignore_globs)?;
        let mru_lock = Arc::new(RwLock::new(mru));
        let mru_clone = Arc::clone(&mru_lock);
        let task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = stop_rx.changed() => {
                        break;
                    }
                    message = event_rx.recv() => {
                        let message = match message {
                            Some(message) => message,
                            None => break,
                        };

                        match message {
                            Ok(event) => {
                                handle_event(&repo_id, &root_path, event, &categorizer, &ignore_matcher, &mru_clone, &recent_store, &event_bus, &logger).await;
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
            let _ = watcher.unwatch(&self.root_path);
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
