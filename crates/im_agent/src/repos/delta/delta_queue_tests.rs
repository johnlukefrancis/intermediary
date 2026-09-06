// Path: crates/im_agent/src/repos/delta/delta_queue_tests.rs
// Description: Settle queue folding, latency ceiling, cap and op-collapse tests

use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::protocol::FileKind;

use super::settle_queue::{PendingOp, SettleQueue};
use super::{DRAIN_BATCH, MAX_LATENCY, QUEUE_CAP, SETTLE_WINDOW};

fn abs(path: &str) -> PathBuf {
    PathBuf::from("/repo").join(path)
}

fn note(queue: &mut SettleQueue, path: &str, op: PendingOp, now: Instant) {
    queue.note(path.to_string(), abs(path), FileKind::Code, op, now);
}

#[test]
fn settle_folds_one_path() {
    let mut queue = SettleQueue::new();
    let start = Instant::now();
    for step in 0..50u64 {
        note(
            &mut queue,
            "src/main.ts",
            PendingOp::Modify,
            start + Duration::from_millis(step * 2),
        );
    }

    assert_eq!(
        queue.len(),
        1,
        "50 marks on one path stay one pending change"
    );
    let last = start + Duration::from_millis(98);
    assert_eq!(queue.next_deadline(), Some(last + SETTLE_WINDOW));
    assert!(queue.drain_due(last, DRAIN_BATCH).is_empty());

    let due = queue.drain_due(last + SETTLE_WINDOW, DRAIN_BATCH);
    assert_eq!(due.len(), 1);
    let change = due.first().expect("one due change");
    assert_eq!(change.folded, 49);
    assert_eq!(change.op, PendingOp::Modify);
    assert!(queue.is_empty());
    assert_eq!(queue.take_dropped().0, 0);
}

#[test]
fn max_latency_forces_emit() {
    let mut queue = SettleQueue::new();
    let start = Instant::now();
    for step in 0..12u64 {
        note(
            &mut queue,
            "src/busy.ts",
            PendingOp::Modify,
            start + Duration::from_millis(step * 60),
        );
    }

    assert_eq!(
        queue.next_deadline(),
        Some(start + MAX_LATENCY),
        "a continuously written path still emits once per MAX_LATENCY",
    );
    let due = queue.drain_due(start + MAX_LATENCY, DRAIN_BATCH);
    assert_eq!(due.len(), 1);
    assert_eq!(due.first().expect("one due change").folded, 11);
}

#[test]
fn cap_counts_dropped() {
    let mut queue = SettleQueue::new();
    let start = Instant::now();
    for index in 0..300u32 {
        note(
            &mut queue,
            &format!("src/f{index}.ts"),
            PendingOp::Add,
            start,
        );
    }

    assert_eq!(queue.len(), QUEUE_CAP);
    let (dropped, paths) = queue.take_dropped();
    assert_eq!(dropped, 300 - QUEUE_CAP as u32);
    assert_eq!(
        paths.first().map(String::as_str),
        Some("src/f256.ts"),
        "the dropped paths come back so their baselines can be evicted",
    );
    assert_eq!(paths.len(), dropped as usize);
    assert_eq!(
        queue.take_dropped(),
        (0, Vec::<String>::new()),
        "taking the counter clears it",
    );

    let due = queue.drain_due(start + SETTLE_WINDOW, DRAIN_BATCH);
    assert_eq!(due.len(), DRAIN_BATCH, "one drain never exceeds the batch");
    assert_eq!(
        due.first().expect("first due").path,
        "src/f0.ts",
        "drain follows first-seen order",
    );
}

#[test]
fn add_then_remove_collapses() {
    let mut queue = SettleQueue::new();
    let start = Instant::now();
    note(&mut queue, "tmp/scratch.ts", PendingOp::Add, start);
    note(
        &mut queue,
        "tmp/scratch.ts",
        PendingOp::Remove,
        start + Duration::from_millis(10),
    );

    let due = queue.drain_due(start + MAX_LATENCY, DRAIN_BATCH);
    assert_eq!(due.len(), 1);
    let change = due.first().expect("one due change");
    assert_eq!(
        change.op,
        PendingOp::Remove,
        "a create-then-delete is a delete"
    );
    assert_eq!(change.folded, 1);

    let mut queue = SettleQueue::new();
    note(&mut queue, "tmp/scratch.ts", PendingOp::Remove, start);
    note(
        &mut queue,
        "tmp/scratch.ts",
        PendingOp::Add,
        start + Duration::from_millis(10),
    );
    let due = queue.drain_due(start + MAX_LATENCY, DRAIN_BATCH);
    assert_eq!(
        due.first().expect("one due change").op,
        PendingOp::Add,
        "a delete-then-create is a create",
    );
}

#[test]
fn rename_moves_baseline() {
    let mut queue = SettleQueue::new();
    let start = Instant::now();
    note(&mut queue, "src/old.ts", PendingOp::Modify, start);
    queue.note_rename(
        "src/old.ts",
        "src/new.ts",
        abs("src/new.ts"),
        FileKind::Code,
        start + Duration::from_millis(20),
    );

    assert_eq!(queue.len(), 1, "both rename arms fold into one change");
    let due = queue.drain_due(start + MAX_LATENCY, DRAIN_BATCH);
    let change = due.first().expect("one due change");
    assert_eq!(change.path, "src/new.ts");
    assert_eq!(
        change.op,
        PendingOp::Rename {
            from: "src/old.ts".to_string()
        },
    );
    assert_eq!(
        change.first_seen, start,
        "MAX_LATENCY still runs from the first sighting"
    );
    assert_eq!(change.folded, 1);
}

/// A re-armed change merging into a mark that landed while it was out inherits
/// the earlier `first_seen`, so its `MAX_LATENCY` ceiling moved: the merged
/// entry's deadline is recomputed from the anchors it now carries.
#[test]
fn requeue_merge_recomputes_the_deadline() {
    let mut queue = SettleQueue::new();
    let start = Instant::now();
    note(&mut queue, "src/main.ts", PendingOp::Modify, start);
    let drained = queue.drain_due(start + MAX_LATENCY, DRAIN_BATCH);
    let change = drained.into_iter().next().expect("one drained change");

    // A fresh mark lands while the drained change is still being read.
    let later = start + Duration::from_millis(400);
    note(&mut queue, "src/main.ts", PendingOp::Modify, later);
    assert_eq!(queue.next_deadline(), Some(later + SETTLE_WINDOW));

    queue.requeue(change, later);
    assert_eq!(
        queue.next_deadline(),
        Some(start + MAX_LATENCY),
        "the merged entry's ceiling runs from the earlier first sighting",
    );
}
