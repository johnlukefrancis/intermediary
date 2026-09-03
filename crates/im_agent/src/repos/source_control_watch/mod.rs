// Path: crates/im_agent/src/repos/source_control_watch/mod.rs
// Description: Watcher-side source control signal: detection, coalescing, git dir resolution

mod coalescer;
mod detector;
mod git_dirs;

use std::time::Instant;

use notify::Event;

use crate::server::EventBus;
use coalescer::{SourceControlSignal, COALESCE_WINDOW};

pub(crate) use detector::SourceControlChangeDetector;
pub(crate) use git_dirs::resolve_external_watches;

/// One repo's source-control signal. The detector decides whether a raw
/// watcher event can move `git status`; the coalescer owns every emission, so
/// a checkout burst becomes one leading and one trailing event.
pub(crate) struct SourceControlWatch {
    detector: SourceControlChangeDetector,
    signal: SourceControlSignal,
}

impl SourceControlWatch {
    pub(crate) fn new(
        repo_id: String,
        event_bus: EventBus,
        detector: SourceControlChangeDetector,
    ) -> Self {
        Self {
            detector,
            signal: SourceControlSignal::new(repo_id, event_bus, COALESCE_WINDOW),
        }
    }

    pub(crate) fn note_event(&self, event: &Event) {
        if self.detector.affects(event) {
            self.signal.mark_dirty();
        }
    }

    /// When the trailing emit falls due, while one is owed.
    pub(crate) fn pending_deadline(&self) -> Option<Instant> {
        self.signal.pending_deadline()
    }

    pub(crate) fn flush(&self) {
        self.signal.flush();
    }
}

#[cfg(test)]
mod tests {
    use crate::logging::{LogConfig, LogLevel, Logger};
    use crate::repos::{RecentFilesStore, RepoWatcher, RepoWatcherConfig};
    use crate::server::EventBus;
    use std::time::Duration;

    /// The whole signal end to end: `.log` files are ignored by the watcher's
    /// file-change path, so every event here comes from the detector, and the
    /// trailing emit comes from the coalescer's timer arm in the watcher task.
    #[tokio::test]
    async fn a_working_tree_burst_emits_one_leading_and_one_trailing_event() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("repo");
        std::fs::create_dir_all(&root).expect("create repo root");
        let logger = Logger::init(LogConfig {
            log_dir: temp.path().join("logs"),
            min_level: LogLevel::Warn,
            emit_stdio: false,
        })
        .await
        .expect("logger");
        let event_bus = EventBus::new(128);
        let mut events = event_bus.subscribe();
        let watcher = RepoWatcher::start(RepoWatcherConfig {
            repo_id: "repo-1".to_string(),
            root_path: root.to_string_lossy().to_string(),
            docs_globs: vec!["**/*.md".to_string()],
            code_globs: vec!["**/*.rs".to_string()],
            ignore_globs: Vec::new(),
            classification_ignore_globs: Vec::new(),
            mru_capacity: 16,
            recent_store: RecentFilesStore::new(temp.path().join("state"), logger.clone()),
            logger: logger.clone(),
            event_bus: event_bus.clone(),
        })
        .await
        .expect("watcher starts");

        for index in 0..50 {
            std::fs::write(root.join(format!("f{index}.log")), b"x").expect("write file");
        }

        let mut emits = 0usize;
        let deadline = tokio::time::Instant::now() + Duration::from_millis(1200);
        while let Ok(Ok(text)) = tokio::time::timeout_at(deadline, events.recv()).await {
            if text.contains("sourceControlChanged") {
                emits += 1;
            }
        }
        watcher.stop().await;

        assert!(
            (1..=3).contains(&emits),
            "a 50 file burst should coalesce, got {emits} events"
        );
    }
}
