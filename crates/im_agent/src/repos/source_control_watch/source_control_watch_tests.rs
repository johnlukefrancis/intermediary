// Path: crates/im_agent/src/repos/source_control_watch/source_control_watch_tests.rs
// Description: SourceControlWatch integration tests — burst coalescing and index-triggered tracked-set reload

use super::{load_tracked_paths, SourceControlChangeDetector, SourceControlWatch, TrackedPathSet};
use crate::logging::{LogConfig, LogLevel, Logger};
use crate::repos::{RecentFilesStore, RepoWatcher, RepoWatcherConfig};
use crate::server::EventBus;
use notify::event::ModifyKind;
use notify::{Event, EventKind};
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

fn git(cwd: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .status()
        .expect("run git");
    assert!(status.success(), "git command failed: {args:?}");
}

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

/// A tracked-set reload runs on an index-change event and the new tree
/// is visible without a second watcher restart.
#[tokio::test]
async fn index_change_event_reloads_the_tracked_set() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join("repo");
    std::fs::create_dir_all(&root).expect("create repo root");
    git(&root, &["init", "-q"]);

    let tracked = TrackedPathSet::empty();
    let detector = SourceControlChangeDetector::new(&root, Vec::new(), &[], tracked.clone())
        .expect("detector builds");
    let logger = Logger::init(LogConfig {
        log_dir: temp.path().join("logs"),
        min_level: LogLevel::Warn,
        emit_stdio: false,
    })
    .await
    .expect("logger");
    let event_bus = EventBus::new(16);
    let watch = SourceControlWatch::new(
        "repo-1".to_string(),
        event_bus,
        detector,
        tracked.clone(),
        root.clone(),
        logger,
    );

    std::fs::write(root.join("new.txt"), b"x").expect("seed file");
    git(&root, &["add", "new.txt"]);
    assert!(
        !tracked.contains("new.txt"),
        "the pre-reload set must not already hold the newly tracked path"
    );

    let index_event = Event::new(EventKind::Modify(ModifyKind::Any))
        .add_path(root.join(".git").join("index"));
    watch.note_event(&index_event);

    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if tracked.contains("new.txt") {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "reload did not pick up the newly tracked path in time"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test]
async fn load_tracked_paths_is_reachable_from_the_module_root() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path();
    git(root, &["init", "-q"]);
    assert!(load_tracked_paths(root).await.expect("load").is_empty());
}
