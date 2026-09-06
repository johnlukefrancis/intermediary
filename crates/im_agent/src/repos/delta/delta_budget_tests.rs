// Path: crates/im_agent/src/repos/delta/delta_budget_tests.rs
// Description: Burst budget tests - per-emit charge, gone budget, causal refill (a flood never refills mid-run, a hot loop does), window close logging

use std::path::PathBuf;
use std::time::{Duration, Instant};

use tempfile::tempdir;

use crate::logging::{LogConfig, LogLevel, Logger};
use crate::protocol::FileKind;

use super::delta_budget::{BurstBucket, Charge};
use super::{
    PendingOp, SettleQueue, BURST_BUDGET, BURST_REFILL_MAX_PENDING, BURST_WINDOW, DRAIN_BATCH,
    GONE_BUDGET,
};

async fn logger(dir: &std::path::Path) -> Logger {
    Logger::init(LogConfig {
        log_dir: dir.join("logs"),
        min_level: LogLevel::Warn,
        emit_stdio: false,
    })
    .await
    .expect("logger")
}

fn spend(bucket: &mut BurstBucket) {
    for _ in 0..BURST_BUDGET {
        assert_eq!(bucket.charge(&PendingOp::Modify), Charge::Resolve);
    }
}

/// The budget no longer looks at the file kind or at whether a baseline is
/// cached: a token is charged for every change that will EMIT, because every
/// emitted delta costs a slot on the 128-slot bus whatever it cost to produce.
#[test]
fn every_emitted_change_costs_a_token() {
    let mut bucket = BurstBucket::new(Instant::now());
    spend(&mut bucket);

    assert_eq!(bucket.charge(&PendingOp::Add), Charge::Withhold);
    assert_eq!(bucket.charge(&PendingOp::Modify), Charge::Withhold);
    assert_eq!(
        bucket.charge(&PendingOp::Rename {
            from: "src/old.rs".to_string(),
        }),
        Charge::Withhold,
    );
    assert_eq!(
        bucket.charge(&PendingOp::Remove),
        Charge::GoneOnly,
        "the one unbudgeted-by-reads outcome: a deletion still prints, as a bare Gone",
    );
}

/// Bare `gone` events have a ceiling of their own: a mass delete past the read
/// budget prints `GONE_BUDGET` of them, then withholds like any other change.
#[test]
fn gone_events_have_their_own_window_budget() {
    let mut bucket = BurstBucket::new(Instant::now());
    spend(&mut bucket);
    for _ in 0..GONE_BUDGET {
        assert_eq!(bucket.charge(&PendingOp::Remove), Charge::GoneOnly);
    }
    assert_eq!(
        bucket.charge(&PendingOp::Remove),
        Charge::Withhold,
        "the {GONE_BUDGET}th bare gone was the last of the window"
    );
}

/// A closing window says whether it denied anything, which is what lets the
/// worker publish the counters instead of stranding them until the next delta.
#[tokio::test]
async fn a_window_that_denied_says_so_on_close() {
    let temp = tempdir().expect("tempdir");
    let logger = logger(temp.path()).await;
    let start = Instant::now();

    let mut quiet = BurstBucket::new(start);
    assert!(
        !quiet.roll(start + BURST_WINDOW, 0, &logger, "repo-1"),
        "a window that denied nothing closes silently",
    );

    let mut spent = BurstBucket::new(start);
    spend(&mut spent);
    assert_eq!(spent.charge(&PendingOp::Modify), Charge::Withhold);
    assert!(
        !spent.roll(start + BURST_WINDOW / 2, 0, &logger, "repo-1"),
        "a window still open is not closed",
    );
    assert!(spent.roll(start + BURST_WINDOW, 0, &logger, "repo-1"));
    assert!(
        !spent.roll(start + BURST_WINDOW + BURST_WINDOW, 0, &logger, "repo-1"),
        "the denial count reset with the window",
    );
}

/// 500 marks in one run - a checkout - resolve exactly `BURST_BUDGET` paths
/// however long the drain takes: the queue stays above
/// `BURST_REFILL_MAX_PENDING` for the whole run, so no window closes mid-run.
#[tokio::test]
async fn a_flood_costs_exactly_one_budget_however_long_it_drains() {
    let temp = tempdir().expect("tempdir");
    let logger = logger(temp.path()).await;
    let start = Instant::now();
    let mut queue = SettleQueue::new();
    for index in 0..500u32 {
        queue.note(
            format!("src/f{index}.ts"),
            PathBuf::from("/repo").join(format!("src/f{index}.ts")),
            FileKind::Code,
            PendingOp::Modify,
            start,
        );
    }
    let mut bucket = BurstBucket::new(start);
    let mut resolved = 0u32;
    let mut withheld = 0u32;
    let mut now = start + BURST_WINDOW;
    // Every wake is a full window apart: time alone would refill sixteen times.
    while queue.len() > 0 {
        bucket.roll(now, queue.len(), &logger, "repo-1");
        for change in queue.drain_due(now, DRAIN_BATCH) {
            match bucket.charge(&change.op) {
                Charge::Resolve => resolved += 1,
                Charge::Withhold => withheld += 1,
                Charge::GoneOnly => panic!("no deletes in this run"),
            }
        }
        now += BURST_WINDOW + Duration::from_secs(1);
    }
    assert_eq!(resolved, BURST_BUDGET);
    assert_eq!(
        resolved + withheld,
        256,
        "everything the queue held was charged once"
    );
    assert!(
        bucket.roll(now, 0, &logger, "repo-1"),
        "once the queue is quiet the next window closes and refills",
    );
}

/// A hot loop over a few files keeps the queue small, so every elapsed window
/// refills: over 6 s of edits to four files nothing is withheld after the first
/// exhausted window.
#[tokio::test]
async fn a_hot_loop_over_a_few_files_refills_every_window() {
    let temp = tempdir().expect("tempdir");
    let logger = logger(temp.path()).await;
    let start = Instant::now();
    let mut bucket = BurstBucket::new(start);
    spend(&mut bucket);
    let hot = 4usize;
    assert!(hot < BURST_REFILL_MAX_PENDING);
    for _ in 0..hot {
        assert_eq!(bucket.charge(&PendingOp::Modify), Charge::Withhold);
    }
    for window in 1..=3u32 {
        let now = start + BURST_WINDOW * window;
        assert!(
            bucket.roll(now, hot, &logger, "repo-1") || window > 1,
            "the exhausted window reports its denials as it closes",
        );
        for _ in 0..hot {
            assert_eq!(
                bucket.charge(&PendingOp::Modify),
                Charge::Resolve,
                "window {window} refilled for the hot loop"
            );
        }
    }
}
