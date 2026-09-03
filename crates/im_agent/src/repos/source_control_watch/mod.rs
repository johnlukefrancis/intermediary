// Path: crates/im_agent/src/repos/source_control_watch/mod.rs
// Description: Watcher-side source control signal: detection, coalescing, git dir resolution, tracked-set reload

mod coalescer;
mod detector;
#[cfg(test)]
mod detector_tests;
mod git_dirs;
#[cfg(test)]
mod source_control_watch_tests;
mod tracked_set;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use notify::Event;

use crate::logging::Logger;
use crate::server::EventBus;
use coalescer::{SourceControlSignal, COALESCE_WINDOW};

pub(crate) use detector::SourceControlChangeDetector;
pub(crate) use git_dirs::resolve_external_watches;
pub(crate) use tracked_set::{load_tracked_paths, TrackedPathSet};

/// Reloads never start closer together than this — a burst of index writes
/// (a rebase, a large `git add`) triggers one `ls-files` run, not one per
/// event.
const RELOAD_DEBOUNCE: Duration = Duration::from_secs(1);

#[derive(Default)]
struct ReloadState {
    last_trigger: Option<Instant>,
    /// Set on a load failure so repeated failures (a moved repo, missing
    /// Git) log once instead of once per debounce tick; cleared on the next
    /// success so a real new failure logs again.
    logged_failure: bool,
}

/// One repo's source-control signal. The detector decides whether a raw
/// watcher event can move `git status`; the coalescer owns every emission, so
/// a checkout burst becomes one leading and one trailing event. This also
/// owns the tracked-path reloader: a `.git/index` change schedules an
/// `ls-files` run (never on the async runtime) that replaces the detector's
/// tracked set once it completes.
pub(crate) struct SourceControlWatch {
    detector: SourceControlChangeDetector,
    signal: SourceControlSignal,
    tracked: TrackedPathSet,
    root_path: PathBuf,
    logger: Logger,
    reload: Arc<Mutex<ReloadState>>,
}

impl SourceControlWatch {
    pub(crate) fn new(
        repo_id: String,
        event_bus: EventBus,
        detector: SourceControlChangeDetector,
        tracked: TrackedPathSet,
        root_path: PathBuf,
        logger: Logger,
    ) -> Self {
        Self {
            detector,
            signal: SourceControlSignal::new(repo_id, event_bus, COALESCE_WINDOW),
            tracked,
            root_path,
            logger,
            reload: Arc::new(Mutex::new(ReloadState::default())),
        }
    }

    pub(crate) fn note_event(&self, event: &Event) {
        if self.detector.affects(event) {
            self.signal.mark_dirty();
        }
        if self.detector.is_index_change(event) {
            self.maybe_reload();
        }
    }

    /// When the trailing emit falls due, while one is owed.
    pub(crate) fn pending_deadline(&self) -> Option<Instant> {
        self.signal.pending_deadline()
    }

    pub(crate) fn flush(&self) {
        self.signal.flush();
    }

    fn maybe_reload(&self) {
        let now = Instant::now();
        {
            let mut state = self.reload_state();
            if let Some(last) = state.last_trigger {
                if now.saturating_duration_since(last) < RELOAD_DEBOUNCE {
                    return;
                }
            }
            state.last_trigger = Some(now);
        }

        let tracked = self.tracked.clone();
        let root_path = self.root_path.clone();
        let logger = self.logger.clone();
        let reload = Arc::clone(&self.reload);
        tokio::spawn(async move {
            match load_tracked_paths(&root_path).await {
                Ok(paths) => {
                    tracked.store(paths);
                    reload.lock().unwrap_or_else(|err| err.into_inner()).logged_failure = false;
                }
                Err(reason) => {
                    let should_log = {
                        let mut state = reload.lock().unwrap_or_else(|err| err.into_inner());
                        let should_log = !state.logged_failure;
                        state.logged_failure = true;
                        should_log
                    };
                    if should_log {
                        logger.warn(
                            "Source control tracked-path reload failed",
                            Some(serde_json::json!({
                                "rootPath": root_path.to_string_lossy(),
                                "reason": reason,
                            })),
                        );
                    }
                }
            }
        });
    }

    fn reload_state(&self) -> std::sync::MutexGuard<'_, ReloadState> {
        // No await happens under this guard; recover rather than unwrap so a
        // poisoned lock cannot take the watcher down (ADR-008).
        self.reload.lock().unwrap_or_else(|err| err.into_inner())
    }
}

