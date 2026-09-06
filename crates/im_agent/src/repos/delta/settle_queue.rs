// Path: crates/im_agent/src/repos/delta/settle_queue.rs
// Description: Pure per-path trailing coalescer for the delta pipeline

use std::path::PathBuf;
use std::time::Instant;

use crate::protocol::FileKind;

use super::{MAX_LATENCY, QUEUE_CAP, SETTLE_WINDOW};

/// What the watcher saw happen to a path, before the delta pipeline resolves it.
/// A rename carries the path it came from so the baseline can move with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PendingOp {
    Add,
    Modify,
    Remove,
    Rename { from: String },
}

/// One path waiting for its quiet window. `first_seen` anchors `MAX_LATENCY`,
/// `last_seen` anchors `SETTLE_WINDOW`, and `deadline` is the earlier of the two.
#[derive(Debug, Clone)]
pub(crate) struct PendingChange {
    pub(crate) path: String,
    pub(crate) abs_path: PathBuf,
    pub(crate) kind: FileKind,
    pub(crate) op: PendingOp,
    pub(crate) first_seen: Instant,
    pub(crate) last_seen: Instant,
    pub(crate) deadline: Instant,
    /// Re-marks folded into this one pending change.
    pub(crate) folded: u32,
    /// Times this change was re-armed because the file was still moving.
    pub(crate) resettles: u32,
}

/// A re-mark of a path already pending collapses onto the pending op rather
/// than queuing a second change: a create-then-delete is a delete, a
/// delete-then-create is a create, and a rename replaces whatever was pending.
fn collapse(existing: &PendingOp, incoming: PendingOp) -> PendingOp {
    match (existing, &incoming) {
        (_, PendingOp::Rename { .. }) => incoming,
        (PendingOp::Add, PendingOp::Remove) => PendingOp::Remove,
        (PendingOp::Remove, PendingOp::Add) => PendingOp::Add,
        (PendingOp::Add, PendingOp::Modify | PendingOp::Add) => PendingOp::Add,
        (PendingOp::Rename { from }, PendingOp::Modify | PendingOp::Add) => {
            PendingOp::Rename { from: from.clone() }
        }
        _ => incoming,
    }
}

/// Trailing coalescer over paths. Every entry point takes `now` so the whole
/// queue is deterministic under test; it performs no IO and never blocks, which
/// is what lets the watcher call it inline on the notify path (ADR-009).
pub(crate) struct SettleQueue {
    /// Ordered by `first_seen`. Bounded by `QUEUE_CAP`, so the linear path scan
    /// is 256 comparisons at worst and no second index has to be kept correct.
    pending: Vec<PendingChange>,
    dropped: u32,
    /// The paths behind `dropped`, bounded by the same `QUEUE_CAP`. The worker
    /// evicts their baselines: a mark that never reached the resolver means the
    /// cached text is no longer what the reader last saw.
    dropped_paths: Vec<String>,
}

impl SettleQueue {
    pub(crate) fn new() -> Self {
        Self {
            pending: Vec::new(),
            dropped: 0,
            dropped_paths: Vec::new(),
        }
    }

    /// Marks a path changed. Folds onto an existing entry when there is one,
    /// otherwise queues a new one - or counts a drop when the queue is full.
    pub(crate) fn note(
        &mut self,
        path: String,
        abs_path: PathBuf,
        kind: FileKind,
        op: PendingOp,
        now: Instant,
    ) {
        if let Some(index) = self.position(&path) {
            let Some(change) = self.pending.get_mut(index) else {
                return;
            };
            change.op = collapse(&change.op, op);
            change.abs_path = abs_path;
            change.kind = kind;
            change.last_seen = now;
            change.folded = change.folded.saturating_add(1);
            change.deadline = deadline_for(change.first_seen, now);
            return;
        }
        if self.pending.len() >= QUEUE_CAP {
            self.dropped = self.dropped.saturating_add(1);
            if self.dropped_paths.len() < QUEUE_CAP {
                self.dropped_paths.push(path);
            }
            return;
        }
        self.pending.push(PendingChange {
            path,
            abs_path,
            kind,
            op,
            first_seen: now,
            last_seen: now,
            deadline: deadline_for(now, now),
            folded: 0,
            resettles: 0,
        });
    }

    /// Folds any change pending on `from` into a rename landing on `to`, so the
    /// two notify arms of one rename produce exactly one pending change.
    pub(crate) fn note_rename(
        &mut self,
        from: &str,
        to: &str,
        abs_to: PathBuf,
        kind: FileKind,
        now: Instant,
    ) {
        let carried = self.position(from).map(|index| self.pending.remove(index));
        self.note(
            to.to_string(),
            abs_to,
            kind,
            PendingOp::Rename {
                from: from.to_string(),
            },
            now,
        );
        let (Some(previous), Some(index)) = (carried, self.position(to)) else {
            return;
        };
        let Some(change) = self.pending.get_mut(index) else {
            return;
        };
        change.first_seen = change.first_seen.min(previous.first_seen);
        change.folded = change
            .folded
            .saturating_add(previous.folded.saturating_add(1));
        change.deadline = deadline_for(change.first_seen, change.last_seen);
        self.pending.sort_by_key(|change| change.first_seen);
    }

    /// Re-arms a drained change whose file was still moving. Already-accepted
    /// work is never dropped at `QUEUE_CAP`: the re-arm count is bounded by
    /// `MAX_RESETTLES` and the in-flight set by `DRAIN_BATCH`.
    pub(crate) fn requeue(&mut self, change: PendingChange, now: Instant) {
        if let Some(index) = self.position(&change.path) {
            if let Some(existing) = self.pending.get_mut(index) {
                existing.resettles = existing.resettles.max(change.resettles.saturating_add(1));
                existing.first_seen = existing.first_seen.min(change.first_seen);
                existing.folded = existing.folded.saturating_add(change.folded);
                // The merged entry inherits the earlier `first_seen`, so its
                // `MAX_LATENCY` ceiling moved: recompute rather than keep a
                // deadline that no longer matches the anchors it was built from.
                existing.deadline = deadline_for(existing.first_seen, existing.last_seen);
            }
            return;
        }
        let mut change = change;
        change.resettles = change.resettles.saturating_add(1);
        change.last_seen = now;
        change.deadline = now + SETTLE_WINDOW;
        let at = self
            .pending
            .iter()
            .position(|other| other.first_seen > change.first_seen)
            .unwrap_or(self.pending.len());
        self.pending.insert(at, change);
    }

    /// The earliest deadline the worker has to wake for.
    pub(crate) fn next_deadline(&self) -> Option<Instant> {
        self.pending.iter().map(|change| change.deadline).min()
    }

    /// Takes up to `max` settled changes in first-seen order.
    pub(crate) fn drain_due(&mut self, now: Instant, max: usize) -> Vec<PendingChange> {
        let mut due = Vec::new();
        let mut index = 0;
        while index < self.pending.len() && due.len() < max {
            let Some(change) = self.pending.get(index) else {
                break;
            };
            if change.deadline <= now {
                due.push(self.pending.remove(index));
            } else {
                index += 1;
            }
        }
        due
    }

    /// Reads and clears the marks discarded at `QUEUE_CAP`: how many, and which
    /// paths (bounded by `QUEUE_CAP`) so their baselines can be evicted.
    pub(crate) fn take_dropped(&mut self) -> (u32, Vec<String>) {
        (
            std::mem::take(&mut self.dropped),
            std::mem::take(&mut self.dropped_paths),
        )
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.pending.len()
    }

    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    fn position(&self, path: &str) -> Option<usize> {
        self.pending.iter().position(|change| change.path == path)
    }
}

/// The quiet window, clamped so a continuously written file still emits once
/// per `MAX_LATENCY`.
fn deadline_for(first_seen: Instant, last_seen: Instant) -> Instant {
    (last_seen + SETTLE_WINDOW).min(first_seen + MAX_LATENCY)
}
