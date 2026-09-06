// Path: crates/im_agent/src/repos/repo_watcher_tests.rs
// Description: Unit tests for the repo watcher's initial-entries ignore filtering and rename delta marks

use super::repo_watcher_startup::filter_initial_entries;
use crate::logging::{LogConfig, LogLevel, Logger};
use crate::protocol::{FileChangeType, FileEntry, FileKind};
use crate::repos::categorizer::Categorizer;
use crate::repos::delta::{DeltaLimits, DeltaService, PendingChange, PendingOp};
use crate::repos::ignore_matcher::IgnoreMatcher;
use crate::repos::mru_index::MruIndex;
use crate::repos::recent_files_store::RecentFilesStore;
use crate::repos::repo_watcher_events::{handle_event, EventContext, PendingRename};
use crate::repos::source_control_watch::{
    SourceControlChangeDetector, SourceControlWatch, TrackedPathSet,
};
use crate::server::{BusMessage, EventBus};
use notify::event::{ModifyKind, RenameMode};
use notify::{Event, EventKind};
use std::path::PathBuf;
use tokio::sync::{broadcast, RwLock};

#[test]
fn test_filter_initial_entries_applies_ignore_globs() {
    let matcher = IgnoreMatcher::new(&["**/*.cpp".to_string()]).expect("valid ignore glob");
    let entry = |path: &str, kind| FileEntry {
        path: path.to_string(),
        kind,
        change_type: FileChangeType::Add,
        mtime: "2026-02-06T00:00:00Z".to_string(),
        size_bytes: Some(8),
        activity: None,
    };
    let entries = vec![
        entry("src\\engine\\render.cpp", FileKind::Code),
        entry("docs/readme.md", FileKind::Docs),
    ];

    let filtered = filter_initial_entries(entries, &matcher);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].path, "docs/readme.md");
}

/// Everything one `EventContext` borrows, built once per test so a test can
/// hand the same context two events - which is what a Windows rename is.
struct EventHarness {
    _temp: tempfile::TempDir,
    root: PathBuf,
    logger: Logger,
    event_bus: EventBus,
    categorizer: Categorizer,
    ignore_matcher: IgnoreMatcher,
    mru: RwLock<MruIndex>,
    recent_store: RecentFilesStore,
    source_control: SourceControlWatch,
    /// Queue only, deliberately: with a worker running, whether a mark is still
    /// on the queue when the test looks would depend on `SETTLE_WINDOW` against
    /// the wall clock, and a stalled runner could drain it first.
    delta: DeltaService,
}

impl EventHarness {
    async fn new() -> Self {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("repo");
        std::fs::create_dir_all(root.join("src")).expect("create repo tree");
        let logger = Logger::init(LogConfig {
            log_dir: temp.path().join("logs"),
            min_level: LogLevel::Warn,
            emit_stdio: false,
        })
        .await
        .expect("logger");
        let event_bus = EventBus::new(16);
        let tracked = TrackedPathSet::empty();
        let detector = SourceControlChangeDetector::new(&root, Vec::new(), &[], tracked.clone())
            .expect("detector");
        Self {
            source_control: SourceControlWatch::new(
                "repo-1".to_string(),
                event_bus.clone(),
                detector,
                tracked,
                root.clone(),
                logger.clone(),
            ),
            delta: DeltaService::new_queue_only(),
            categorizer: Categorizer::new(&["**/*.md".to_string()], &["**/*.rs".to_string()])
                .expect("globs"),
            ignore_matcher: IgnoreMatcher::new(&[]).expect("ignore matcher"),
            mru: RwLock::new(MruIndex::new(8).expect("mru")),
            recent_store: RecentFilesStore::new(temp.path().join("state"), logger.clone()),
            logger,
            event_bus,
            root,
            _temp: temp,
        }
    }

    fn context(&self) -> EventContext<'_> {
        EventContext {
            repo_id: "repo-1",
            root_path: &self.root,
            categorizer: &self.categorizer,
            ignore_matcher: &self.ignore_matcher,
            mru: &self.mru,
            recent_store: &self.recent_store,
            event_bus: &self.event_bus,
            logger: &self.logger,
            source_control: &self.source_control,
            delta: &self.delta,
            pending_rename: PendingRename::new(),
        }
    }

    /// One raw rename arm through the real `handle_event`.
    async fn rename(&self, context: &EventContext<'_>, mode: RenameMode, paths: &[&str]) {
        let event = paths.iter().fold(
            Event::new(EventKind::Modify(ModifyKind::Name(mode))),
            |event, path| event.add_path(self.root.join(path)),
        );
        handle_event(context, event).await;
    }
}

/// One rename mark, whichever arm the backend used.
fn assert_renamed(pending: &[PendingChange], from: &str, to: &str) {
    assert_eq!(pending.len(), 1, "one rename mark, not unlink + add");
    let change = pending.first().expect("pending rename");
    assert_eq!(change.path, to);
    assert_eq!(change.op, PendingOp::Rename { from: from.into() });
}

/// `fileChanged` is untouched by the delta marks: unlink for the source, then
/// add for the destination, exactly as Auto Files has always received them.
fn assert_unlink_then_add(events: &mut broadcast::Receiver<BusMessage>) {
    let mut seen = Vec::new();
    while let Ok(text) = events.try_recv() {
        if text.contains("\"fileChanged\"") {
            seen.push(text);
        }
    }
    assert_eq!(seen.len(), 2, "{seen:?}");
    assert!(seen[0].contains("\"unlink\"") && seen[0].contains("src/a.rs"));
    assert!(seen[1].contains("\"add\"") && seen[1].contains("src/b.rs"));
}

/// Every rename arm through the real `handle_event` marks exactly one delta:
/// the two-path `Both` arm, the Windows `From`-then-`To` pair (the
/// ReadDirectoryChangesW backend splits one rename across two events, and the
/// pending Remove must fold into the Rename), and a lone `From`, which stays
/// the plain delete it has always been.
#[tokio::test]
async fn rename_arms_mark_exactly_one_delta() {
    let harness = EventHarness::new().await;
    std::fs::write(harness.root.join("src/b.rs"), "fn main() {}\n").expect("seed destination");
    let mut events = harness.event_bus.subscribe();
    let context = harness.context();

    let both = &["src/a.rs", "src/b.rs"][..];
    let (from, to) = (&["src/a.rs"][..], &["src/b.rs"][..]);
    harness.rename(&context, RenameMode::Both, both).await;
    assert_renamed(&harness.delta.take_pending(), "src/a.rs", "src/b.rs");
    assert_unlink_then_add(&mut events);

    harness.rename(&context, RenameMode::From, from).await;
    harness.rename(&context, RenameMode::To, to).await;
    assert_renamed(&harness.delta.take_pending(), "src/a.rs", "src/b.rs");
    assert_unlink_then_add(&mut events);

    harness.rename(&context, RenameMode::From, from).await;
    let lone = harness.delta.take_pending();
    assert_eq!(lone.len(), 1, "a From with no To is a delete");
    assert_eq!(lone[0].op, PendingOp::Remove);
}

/// End to end through a real watcher: one settled write to a tracked file
/// publishes exactly one `fileDelta`, measured against the index on the first
/// sighting, with the modified lines in the patch.
#[tokio::test]
async fn settled_write_publishes_one_file_delta() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join("repo");
    std::fs::create_dir_all(root.join("src")).expect("create repo tree");
    let git = |args: &[&str]| {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(&root)
            .status()
            .expect("run git");
        assert!(status.success(), "git command failed: {args:?}");
    };
    git(&["init", "-q"]);
    std::fs::write(root.join("src/a.rs"), "fn a() {}\nfn b() {}\n").expect("seed");
    git(&["add", "src/a.rs"]);
    let logger = Logger::init(LogConfig {
        log_dir: temp.path().join("logs"),
        min_level: LogLevel::Warn,
        emit_stdio: false,
    })
    .await
    .expect("logger");
    let event_bus = EventBus::new(64);
    let mut events = event_bus.subscribe();
    let watcher = crate::repos::RepoWatcher::start(crate::repos::RepoWatcherConfig {
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
        delta_limits: DeltaLimits::new(),
    })
    .await
    .expect("watcher starts");

    std::fs::write(root.join("src/a.rs"), "fn a() {}\nfn c() {}\n").expect("modify");

    let mut deltas = Vec::new();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(1500);
    while let Ok(Ok(text)) = tokio::time::timeout_at(deadline, events.recv()).await {
        if text.contains("\"fileDelta\"") {
            deltas.push(text);
        }
    }
    watcher.stop().await;

    assert_eq!(deltas.len(), 1, "one settled write, one delta: {deltas:?}");
    let delta = &deltas[0];
    for expected in [
        "\"seq\":1",
        "\"path\":\"src/a.rs\"",
        "\"op\":\"modify\"",
        "\"tracked\":true",
        "\"baseline\":\"index\"",
        "-fn b() {}\\n+fn c() {}\\n",
    ] {
        assert!(delta.contains(expected), "missing {expected} in {delta}");
    }
}
