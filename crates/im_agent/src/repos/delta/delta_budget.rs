// Path: crates/im_agent/src/repos/delta/delta_budget.rs
// Description: The delta read budget - burst token bucket, causal refill, gone budget, per-window log gates, and the per-change charge decision

use std::collections::HashSet;
use std::time::Instant;

use crate::logging::Logger;

use super::{
    PendingOp, BURST_BUDGET, BURST_REFILL_MAX_PENDING, BURST_WINDOW, DRAIN_BATCH, GONE_BUDGET,
};

/// What the budget allows for one settled change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Charge {
    /// Resolve normally: the resolver may read the file and spawn `git show`.
    Resolve,
    /// A change with no token left: no event at all, and the baseline is evicted
    /// so the path's next sighting says `VS INDEX`. The UI's burst card is what
    /// tells the reader those edits happened.
    Withhold,
    /// A delete with no read token left but a `GONE_BUDGET` token: the deletion
    /// still reaches the UI as `Gone` with no patch, no read and no `git show`.
    /// This is the ONE outcome that emits without a read token - a `Gone`
    /// event is a handful of bytes - and it has its own per-window ceiling so a
    /// mass delete cannot turn into a mass of bus slots.
    GoneOnly,
}

/// Text reads allowed per `BURST_WINDOW`, plus the per-window log gates: one
/// `info` when a window that denied anything closes, one `warn` per path.
pub(super) struct BurstBucket {
    window_start: Instant,
    spent: u32,
    gone_spent: u32,
    denied: u32,
    warned: HashSet<String>,
}

impl BurstBucket {
    pub(super) fn new(now: Instant) -> Self {
        Self {
            window_start: now,
            spent: 0,
            gone_spent: 0,
            denied: 0,
            warned: HashSet::new(),
        }
    }

    /// Closes an elapsed window, logging it once when it denied anything.
    /// The refill is causal: a window closes only once `BURST_WINDOW` has
    /// elapsed AND fewer than `BURST_REFILL_MAX_PENDING` marks are pending,
    /// so a checkout's run never refills mid-run while a hot loop over a few
    /// files still refills every window. Returns true when the window that
    /// just closed denied something, so the worker can publish the counters
    /// instead of stranding them until the next emitted delta.
    pub(super) fn roll(
        &mut self,
        now: Instant,
        pending: usize,
        logger: &Logger,
        repo_id: &str,
    ) -> bool {
        if now.saturating_duration_since(self.window_start) < BURST_WINDOW
            || pending >= BURST_REFILL_MAX_PENDING
        {
            return false;
        }
        let denied = self.denied > 0;
        if self.denied > 0 {
            logger.info(
                "Delta burst budget withheld paths",
                Some(serde_json::json!({
                    "repoId": repo_id,
                    "withheld": self.denied,
                    "window": BURST_WINDOW.as_millis() as u64,
                })),
            );
        }
        *self = Self::new(now);
        denied
    }

    /// Charges one change against the window. A token is charged for every
    /// change that will EMIT, not only for the ones that read: an image costs
    /// one `metadata` call and a cached delete costs none, but both still
    /// publish an event onto a 128-slot bus that the burst budget exists to
    /// keep bounded. A delete past the read budget draws on `GONE_BUDGET`
    /// instead - see `Charge::GoneOnly`.
    pub(super) fn charge(&mut self, op: &PendingOp) -> Charge {
        if self.take() {
            return Charge::Resolve;
        }
        if matches!(op, PendingOp::Remove) && self.gone_spent < GONE_BUDGET {
            self.gone_spent = self.gone_spent.saturating_add(1);
            return Charge::GoneOnly;
        }
        Charge::Withhold
    }

    /// Spends one read token, or counts the denial for this window's one log line.
    fn take(&mut self) -> bool {
        if self.spent < BURST_BUDGET {
            self.spent = self.spent.saturating_add(1);
            return true;
        }
        self.denied = self.denied.saturating_add(1);
        false
    }

    /// True the first time a path fails inside this window. The set is bounded
    /// by the reads this window can perform plus the deletes it saw.
    pub(super) fn first_failure(&mut self, path: &str) -> bool {
        if self.warned.contains(path) || self.warned.len() >= DRAIN_BATCH * (BURST_BUDGET as usize)
        {
            return false;
        }
        self.warned.insert(path.to_string());
        true
    }
}
