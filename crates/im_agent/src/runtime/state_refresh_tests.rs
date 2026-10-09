// Path: crates/im_agent/src/runtime/state_refresh_tests.rs
// Description: Refresh rejects stopped watchers without publishing retained history

use crate::logging::{LogConfig, LogLevel, Logger};
use crate::repos::{DeltaLimits, RecentFilesStore, RepoWatcher, RepoWatcherConfig};
use crate::server::EventBus;

use super::AgentRuntime;

#[tokio::test]
async fn refresh_rejects_a_retained_stopped_watcher_without_a_snapshot() {
    let temp = tempfile::tempdir().expect("tempdir");
    let logger = Logger::init(LogConfig {
        log_dir: temp.path().join("logs"),
        min_level: LogLevel::Error,
        emit_stdio: false,
    })
    .await
    .expect("logger");
    let event_bus = EventBus::new(8);
    let watcher = RepoWatcher::start(RepoWatcherConfig {
        repo_id: "repo".into(),
        root_path: temp.path().to_string_lossy().into_owned(),
        docs_globs: Vec::new(),
        code_globs: Vec::new(),
        ignore_globs: Vec::new(),
        classification_ignore_globs: Vec::new(),
        mru_capacity: 8,
        recent_store: RecentFilesStore::new(temp.path().join("state"), logger.clone()),
        logger,
        event_bus: event_bus.clone(),
        delta_limits: DeltaLimits::new(),
    })
    .await
    .expect("watcher");
    watcher.stop().await;
    tokio::task::yield_now().await;
    assert!(watcher.is_task_finished());
    let mut runtime = AgentRuntime::new();
    runtime.watchers.insert("repo".into(), watcher);
    let mut events = event_bus.subscribe();
    let error = runtime
        .refresh_repo("repo")
        .await
        .expect_err("stopped watcher");
    assert_eq!(error.code(), "WATCHER_STOPPED");
    tokio::task::yield_now().await;
    assert!(matches!(
        events.try_recv(),
        Err(tokio::sync::broadcast::error::TryRecvError::Empty)
    ));
}
