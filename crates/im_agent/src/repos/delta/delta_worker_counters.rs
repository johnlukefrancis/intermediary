// Path: crates/im_agent/src/repos/delta/delta_worker_counters.rs
// Description: The withheld and dropped counters a worker owes the UI, the one sequence they share with fileDelta, and when a standalone publish is due

use std::time::Instant;

use super::BURST_WINDOW;

/// Counters ride the next `fileDelta`, or a `fileDeltaCounters` event when no
/// delta is coming. `seq` lives here because both carriers spend it: a
/// counters event consumes a number like a delta, so losing either is a gap.
pub(super) struct DeltaCounters {
    seq: u64,
    withheld: u32,
    dropped: u32,
    /// When counters last left on either carrier; the time-bound trigger in
    /// `standalone_due` measures from here.
    published_at: Instant,
}

/// One carrier's worth: the next sequence number and the counters, taken.
pub(super) struct CountersTaken {
    pub(super) seq: u64,
    pub(super) withheld: u32,
    pub(super) dropped: u32,
}

impl DeltaCounters {
    pub(super) fn new(now: Instant) -> Self {
        Self {
            seq: 0,
            withheld: 0,
            dropped: 0,
            published_at: now,
        }
    }

    pub(super) fn note_withheld(&mut self) {
        self.withheld = self.withheld.saturating_add(1);
    }

    pub(super) fn note_dropped(&mut self, count: u32) {
        self.dropped = self.dropped.saturating_add(count);
    }

    pub(super) fn is_zero(&self) -> bool {
        self.withheld == 0 && self.dropped == 0
    }

    /// Spends the next sequence number and hands over the counters, reset.
    pub(super) fn take(&mut self, now: Instant) -> CountersTaken {
        self.seq = self.seq.saturating_add(1);
        self.published_at = now;
        CountersTaken {
            seq: self.seq,
            withheld: std::mem::take(&mut self.withheld),
            dropped: std::mem::take(&mut self.dropped),
        }
    }

    /// Whether non-zero counters should go out on their own now. Three
    /// triggers, any one enough: the queue went quiet (no delta is coming to
    /// carry them), a window that denied something closed, or `BURST_WINDOW`
    /// has passed since counters last left - so a run that keeps the queue
    /// busy and the bucket unrefilled still reports its withheld paths every
    /// window instead of stranding them until the run ends.
    pub(super) fn standalone_due(&self, now: Instant, window_denied: bool, quiet: bool) -> bool {
        if self.is_zero() {
            return false;
        }
        window_denied || quiet || now.saturating_duration_since(self.published_at) >= BURST_WINDOW
    }
}
