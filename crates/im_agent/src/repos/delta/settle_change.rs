// Path: crates/im_agent/src/repos/delta/settle_change.rs
// Description: One pending change on the settle queue - its op vocabulary, op collapse, and deadline arithmetic

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use crate::protocol::FileKind;

use super::{MAX_LATENCY, SETTLE_WINDOW};

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
    /// The index blob captured before the first settled read of an uncached
    /// path: `None` until fetched, then `Some(None)` (nothing at stage 0) or
    /// `Some(text)`. A re-settle diffs against this capture rather than
    /// re-reading an index a commit may have moved in between.
    pub(crate) index_baseline: Option<Option<Arc<str>>>,
}

/// A re-mark of a path already pending collapses onto the pending op rather
/// than queuing a second change: a create-then-delete is a delete, a
/// delete-then-create is a create, and a rename replaces whatever was pending.
pub(super) fn collapse(existing: &PendingOp, incoming: PendingOp) -> PendingOp {
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

/// The quiet window, clamped so a continuously written file still emits once
/// per `MAX_LATENCY`.
pub(super) fn deadline_for(first_seen: Instant, last_seen: Instant) -> Instant {
    (last_seen + SETTLE_WINDOW).min(first_seen + MAX_LATENCY)
}
