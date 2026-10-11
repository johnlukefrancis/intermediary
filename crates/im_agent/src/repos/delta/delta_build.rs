// Path: crates/im_agent/src/repos/delta/delta_build.rs
// Description: The blocking half of a text delta - diff settled content against its baseline and derive the baseline retained next

use std::sync::Arc;
use std::time::Instant;

use super::baseline::{fingerprints, split_lines};
use super::unified_patch::diff_against;
use super::{all_added_patch, Baseline, PatchOutput, MAX_DELTA_FILE_BYTES};

pub(super) struct BuiltDelta {
    pub(super) patch: PatchOutput,
    /// `None` when the content has no retainable form (too many lines to fingerprint).
    pub(super) retained: Option<Baseline>,
}

/// Blocking CPU over at most `MAX_FINGERPRINT_FILE_BYTES`; the caller runs it
/// inside `spawn_blocking` under a read permit. Large content retains fingerprints.
pub(super) fn build_delta(old: Option<&Baseline>, new: Arc<str>, deadline: Instant) -> BuiltDelta {
    let new_lines = split_lines(&new);
    let large = new.len() as u64 > MAX_DELTA_FILE_BYTES;
    let new_prints = if large {
        fingerprints(&new_lines)
    } else {
        None
    };
    let patch = match old {
        Some(old) => diff_against(old, &new_lines, new_prints.as_deref(), deadline),
        None => all_added_patch(&new),
    };
    let retained = if large {
        new_prints.map(Baseline::Lines)
    } else {
        Some(Baseline::Text(Arc::clone(&new)))
    };
    BuiltDelta { patch, retained }
}
