// Path: crates/im_agent/src/repos/delta/delta_worker_evict.rs
// Description: Baseline eviction for changes that never reached the resolver - dropped marks (bounded record, overflow clears all) and withheld changes

use std::collections::HashSet;

use crate::logging::Logger;

use super::{BaselineCache, PendingChange, PendingOp};

/// A mark discarded at `QUEUE_CAP` never reached the resolver, so the cached
/// text is no longer what the reader last saw. When the record of which paths
/// that was has itself overflowed, no entry can be trusted: the whole cache
/// goes, and every next sighting says `VS INDEX`.
pub(super) struct DroppedEviction {
    /// True while consecutive wakes keep overflowing the record, so one flood
    /// logs the cache clear once rather than once per wake.
    overflow_logged: bool,
}

impl DroppedEviction {
    pub(super) fn new() -> Self {
        Self {
            overflow_logged: false,
        }
    }

    pub(super) fn apply(
        &mut self,
        cache: &mut BaselineCache,
        dropped_paths: &HashSet<String>,
        overflowed: bool,
        logger: &Logger,
        repo_id: &str,
    ) {
        if !overflowed {
            self.overflow_logged = false;
            for path in dropped_paths {
                cache.remove(path);
            }
            return;
        }
        cache.clear();
        if !self.overflow_logged {
            logger.warn(
                "Delta dropped-path record overflowed; baseline cache cleared",
                Some(serde_json::json!({ "repoId": repo_id })),
            );
        }
        self.overflow_logged = true;
    }
}

/// A withheld change never reached the resolver, so the text cached for it is
/// no longer what the reader last saw. A withheld rename leaves BOTH endpoints
/// stale: the source's baseline was never carried across and the destination
/// was never read.
pub(super) fn evict_withheld(cache: &mut BaselineCache, change: &PendingChange) {
    if let PendingOp::Rename { from } = &change.op {
        cache.remove(from);
    }
    cache.remove(&change.path);
}
