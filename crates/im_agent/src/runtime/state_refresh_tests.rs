// Path: crates/im_agent/src/runtime/state_refresh_tests.rs
// Description: Refresh rejects stopped watchers; watch_repo replaces one whose root watch failed

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
    assert!(watcher.needs_restart());
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

async fn wait_for(deadline_ms: u64, mut done: impl FnMut() -> bool) {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(deadline_ms);
    while !done() {
        assert!(
            tokio::time::Instant::now() < deadline,
            "condition not reached in time"
        );
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
}

/// A failed root registration used to leave a watcher that looked live forever.
#[tokio::test]
async fn watch_repo_restarts_a_watcher_whose_root_registration_failed() {
    let temp = tempfile::tempdir().expect("tempdir");
    let logger = Logger::init(LogConfig {
        log_dir: temp.path().join("logs"),
        min_level: LogLevel::Error,
        emit_stdio: false,
    })
    .await
    .expect("logger");
    let event_bus = EventBus::new(64);
    let store = RecentFilesStore::new(temp.path().join("state"), logger.clone());
    let repo_root = temp.path().join("repo");
    std::fs::create_dir(&repo_root).expect("repo root");
    let failed = RepoWatcher::start(RepoWatcherConfig {
        repo_id: "repo".into(),
        root_path: temp.path().join("missing").to_string_lossy().into_owned(),
        docs_globs: Vec::new(),
        code_globs: Vec::new(),
        ignore_globs: Vec::new(),
        classification_ignore_globs: Vec::new(),
        mru_capacity: 8,
        recent_store: store.clone(),
        logger: logger.clone(),
        event_bus: event_bus.clone(),
        delta_limits: DeltaLimits::new(),
    })
    .await
    .expect("watcher starts; registration fails in the background");
    wait_for(2000, || failed.needs_restart()).await;

    let mut runtime = AgentRuntime::new();
    runtime.recent_files_store = Some(store);
    let repo = serde_json::json!({"repoId": "repo", "root": {"kind": "wsl", "path": repo_root}});
    runtime.repo_configs.insert(
        "repo".into(),
        serde_json::from_value(repo).expect("repo config"),
    );
    runtime.watchers.insert("repo".into(), failed);

    runtime
        .watch_repo("repo", &event_bus, &logger)
        .await
        .expect("watch_repo restarts the watcher");
    let replacement = runtime.watchers.get("repo").expect("replacement watcher");
    wait_for(2000, || replacement.registration_finished()).await;
    assert!(
        !replacement.needs_restart(),
        "the replacement observes the real root"
    );
    assert_eq!(runtime.watched_repo_ids(), vec!["repo".to_string()]);
}
