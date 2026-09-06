// Path: crates/im_agent/src/repos/delta/delta_worker_emit.rs
// Description: Builds the fileDelta wire event from one resolved change and the worker's stamp

use crate::protocol::{DeltaOp, DeltaPayload, FileDeltaEvent};

use super::{PendingChange, PendingOp};

/// What the worker stamps onto one delta beyond the change itself. The
/// counters arrive already taken, so the worker owns exactly when they reset.
pub(super) struct DeltaStamp {
    pub(super) repo_id: String,
    pub(super) seq: u64,
    pub(super) tracked: bool,
    pub(super) withheld: u32,
    pub(super) dropped: u32,
}

/// One resolved change as the additive `fileDelta` event.
pub(super) fn file_delta_event(
    stamp: DeltaStamp,
    change: PendingChange,
    payload: DeltaPayload,
    mtime: String,
) -> FileDeltaEvent {
    let (op, from_path) = match change.op {
        PendingOp::Add => (DeltaOp::Add, None),
        PendingOp::Modify => (DeltaOp::Modify, None),
        PendingOp::Remove => (DeltaOp::Remove, None),
        PendingOp::Rename { from } => (DeltaOp::Rename, Some(from)),
    };
    FileDeltaEvent {
        repo_id: stamp.repo_id,
        seq: stamp.seq,
        tracked: Some(stamp.tracked),
        path: change.path,
        from_path,
        kind: change.kind,
        op,
        mtime,
        folded: change.folded,
        withheld: stamp.withheld,
        dropped: stamp.dropped,
        payload,
    }
}

impl FileDeltaEvent {
    /// The bytes this event's body stands for, for the debug line: patch
    /// length for text, the file size for an image or an opaque, nothing for gone.
    pub(crate) fn payload_size(&self) -> u64 {
        match &self.payload {
            DeltaPayload::Text { patch, .. } => patch.len() as u64,
            DeltaPayload::Image { bytes, .. } | DeltaPayload::Opaque { bytes, .. } => *bytes,
            DeltaPayload::Gone => 0,
        }
    }
}
