// Path: crates/im_agent/src/repos/delta/delta_worker_tests.rs
// Description: Worker-level tests - a withheld rename evicts both baselines, counters consume the shared seq, the time-bound standalone counters trigger

use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use im_bundle::cancel::BundleCancelToken;
use tempfile::tempdir;
use tokio::sync::{watch, Notify};

use crate::logging::{LogConfig, LogLevel, Logger};
use crate::protocol::FileKind;
use crate::repos::source_control_watch::TrackedPathSet;
use crate::server::EventBus;

use super::delta_worker::{DeltaWorker, WorkerLinks};
use super::delta_worker_counters::DeltaCounters;
use super::delta_worker_evict::evict_withheld;
use super::{
    BaselineCache, DeltaLimits, PendingChange, PendingOp, SettleQueue, BURST_BUDGET, BURST_WINDOW,
};

fn change(path: &str, abs: std::path::PathBuf, kind: FileKind, op: PendingOp) -> PendingChange {
    let now = Instant::now();
    PendingChange {
        path: path.to_string(),
        abs_path: abs,
        kind,
        op,
        first_seen: now,
        last_seen: now,
        deadline: now,
        folded: 0,
        resettles: 0,
        index_baseline: None,
    }
}

/// A withheld rename never carried its baseline across and never read the
/// destination, so BOTH endpoints are evicted; a withheld modify evicts one.
#[test]
fn a_withheld_rename_evicts_both_baselines() {
    let mut cache = BaselineCache::new(4096);
    cache.insert("src/old.rs".to_string(), Arc::from("a\n"));
    cache.insert("src/new.rs".to_string(), Arc::from("b\n"));
    cache.insert("src/other.rs".to_string(), Arc::from("c\n"));

    let rename = change(
        "src/new.rs",
        "/repo/src/new.rs".into(),
        FileKind::Code,
        PendingOp::Rename {
            from: "src/old.rs".to_string(),
        },
    );
    evict_withheld(&mut cache, &rename);
    assert!(cache.get("src/old.rs").is_none(), "the source went");
    assert!(cache.get("src/new.rs").is_none(), "the destination went");
    assert!(cache.get("src/other.rs").is_some(), "nothing else did");
}

/// `fileDeltaCounters` spends a number from the same sequence as `fileDelta`,
/// so a counters event the transport loses is a visible gap like any other.
#[tokio::test]
async fn counters_consume_the_shared_sequence() {
    let temp = tempdir().expect("tempdir");
    let root = temp.path().join("repo");
    fs::create_dir_all(&root).expect("repo dir");
    let logger = Logger::init(LogConfig {
        log_dir: temp.path().join("logs"),
        min_level: LogLevel::Warn,
        emit_stdio: false,
    })
    .await
    .expect("logger");
    let event_bus = EventBus::new(256);
    let mut events = event_bus.subscribe();
    let (_stop_tx, stop) = watch::channel(false);
    let mut worker = DeltaWorker::new(
        "repo-1".to_string(),
        root.clone(),
        event_bus,
        logger,
        TrackedPathSet::empty(),
        DeltaLimits::new(),
        WorkerLinks {
            queue: Arc::new(Mutex::new(SettleQueue::new())),
            nudge: Arc::new(Notify::new()),
            stop,
            cancel: BundleCancelToken::new(),
        },
    );

    // Images are metadata-only, so spending the budget costs no reads and no git.
    for index in 0..=BURST_BUDGET {
        let name = format!("shot{index}.png");
        let abs = root.join(&name);
        fs::write(&abs, [0_u8; 4]).expect("image");
        worker
            .process(change(&name, abs, FileKind::Image, PendingOp::Add))
            .await;
    }
    worker.flush_counters();

    let mut seen = Vec::new();
    while let Ok(text) = events.try_recv() {
        seen.push(text);
    }
    assert_eq!(seen.len() as u32, BURST_BUDGET + 1, "{seen:?}");
    let last = seen.last().expect("the counters event");
    assert!(last.contains("\"type\":\"fileDeltaCounters\""), "{last}");
    assert!(last.contains("\"withheld\":1"), "{last}");
    assert!(
        last.contains(&format!("\"seq\":{}", BURST_BUDGET + 1)),
        "the counters event took the next number: {last}"
    );
    assert!(
        seen[BURST_BUDGET as usize - 1].contains(&format!("\"seq\":{BURST_BUDGET}")),
        "the last delta held the number before it"
    );
}

/// Counters go out on their own when the queue is quiet, when a denying
/// window closed, or - independent of the refill rule - once `BURST_WINDOW`
/// has passed since counters last left on any carrier. Zero counters are
/// never due, and a take resets the clock.
#[test]
fn standalone_counters_are_due_on_a_time_bound_of_their_own() {
    let start = Instant::now();
    let mut counters = DeltaCounters::new(start);
    let later = start + BURST_WINDOW;
    assert!(
        !counters.standalone_due(later, true, true),
        "zero counters are never due"
    );

    counters.note_withheld();
    let busy = start + BURST_WINDOW - Duration::from_millis(1);
    assert!(
        !counters.standalone_due(busy, false, false),
        "busy, inside the window"
    );
    assert!(
        counters.standalone_due(busy, true, false),
        "a denying window closed"
    );
    assert!(
        counters.standalone_due(busy, false, true),
        "the queue went quiet"
    );
    assert!(
        counters.standalone_due(later, false, false),
        "the window elapsed while the queue stayed busy"
    );

    let taken = counters.take(later);
    assert_eq!((taken.seq, taken.withheld, taken.dropped), (1, 1, 0));
    counters.note_dropped(2);
    assert!(
        !counters.standalone_due(later + BURST_WINDOW / 2, false, false),
        "the take restarted the clock"
    );
    assert!(counters.standalone_due(later + BURST_WINDOW, false, false));
    assert_eq!(counters.take(later + BURST_WINDOW).seq, 2);
}
