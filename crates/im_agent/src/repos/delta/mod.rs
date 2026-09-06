// Path: crates/im_agent/src/repos/delta/mod.rs
// Description: Bounded delta pipeline for fileDelta - named bounds, DeltaService owner, pure-core exports

use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use im_bundle::cancel::BundleCancelToken;
use tokio::sync::{watch, Notify};

use crate::logging::Logger;
use crate::protocol::FileKind;
use crate::repos::source_control_watch::TrackedPathSet;
use crate::server::EventBus;

mod baseline_cache;
mod delta_budget;
mod delta_limits;
mod delta_read;
mod delta_reads;
mod delta_resolve;
mod delta_resolve_text;
mod delta_stamp;
mod delta_worker;
mod delta_worker_counters;
mod delta_worker_emit;
mod delta_worker_evict;
mod settle_change;
mod settle_queue;
mod unified_patch;

#[cfg(test)]
mod delta_budget_tests;
#[cfg(test)]
mod delta_cache_tests;
#[cfg(test)]
mod delta_order_tests;
#[cfg(test)]
mod delta_patch_tests;
#[cfg(test)]
mod delta_queue_tests;
#[cfg(test)]
mod delta_resolve_tests;
#[cfg(test)]
mod delta_resolve_text_tests;
#[cfg(test)]
mod delta_test_support;
#[cfg(test)]
mod delta_worker_tests;

pub(crate) use baseline_cache::BaselineCache;
pub use delta_limits::DeltaLimits;
pub(crate) use delta_read::{read_settled, ReadOutcome};
pub(crate) use settle_change::{PendingChange, PendingOp};
pub(crate) use settle_queue::SettleQueue;
pub(crate) use unified_patch::{all_added_patch, all_removed_patch, compute_patch, PatchOutput};

/// Quiet time a path must hold before its delta is read: long enough to swallow
/// an editor's write-truncate-rename dance, short enough to stay under the
/// 150-300 ms card budget.
pub(crate) const SETTLE_WINDOW: Duration = Duration::from_millis(120);

/// A path that keeps being written can never starve: the deadline never moves
/// past `first_seen + MAX_LATENCY`, so a busy file still prints twice a second.
pub(crate) const MAX_LATENCY: Duration = Duration::from_millis(500);

/// Distinct paths held pending at once. Past this the mark is counted in
/// `dropped` and discarded - the queue never grows with a checkout.
pub(crate) const QUEUE_CAP: usize = 256;

/// Paths taken off the queue per wake, so one drain can never monopolise the worker.
pub(crate) const DRAIN_BATCH: usize = 16;

/// Re-arms allowed for one still-moving file before its content is accepted as
/// read; bounds the truncate-then-write retry at ~3 x SETTLE_WINDOW.
pub(crate) const MAX_RESETTLES: u32 = 3;

/// Largest file the stream reads or diffs; above this the payload is `Opaque(tooLarge)`.
pub(crate) const MAX_DELTA_FILE_BYTES: u64 = 512 * 1024;

/// Baseline text retained per repo. Bounds agent memory at repos x 16 MiB.
pub(crate) const CACHE_BYTES_PER_REPO: usize = 16 * 1024 * 1024;

/// Wall-clock budget handed to `similar`; past it the algorithm approximates
/// rather than blocking the blocking-pool thread.
pub(crate) const DIFF_DEADLINE: Duration = Duration::from_millis(150);

/// Context lines around each hunk - the unified-diff default the viewer expects.
pub(crate) const CONTEXT_RADIUS: usize = 3;

/// Largest patch published for one delta. Keeps the 128-slot event bus's worst
/// case near 9 MiB and the card's parse cost flat.
pub(crate) const PATCH_MAX_BYTES: usize = 64 * 1024;

/// Concurrent settled reads across the whole agent process, so the stream can
/// never contend with the user's own IO.
pub(crate) const DELTA_READ_CONCURRENCY: usize = 2;

/// Text deltas read per `BURST_WINDOW`; beyond it paths are counted in
/// `withheld` and evicted, so a 500-file checkout costs a bounded 32 reads.
pub(crate) const BURST_BUDGET: u32 = 32;

/// The window `BURST_BUDGET` refills over.
pub(crate) const BURST_WINDOW: Duration = Duration::from_secs(2);

/// The bucket refills only while fewer than this many marks are pending: one
/// drain's worth. A checkout's run holds the queue above it for its whole
/// life, so the run costs `BURST_BUDGET` reads however long it takes, while a
/// hot loop over a few files still refills every window.
pub(crate) const BURST_REFILL_MAX_PENDING: usize = DRAIN_BATCH;

/// Bare `gone` events allowed per `BURST_WINDOW` once the read budget is
/// spent. Each is a handful of bytes, but a 10k-file delete must not become
/// 10k bus slots; past this a delete is withheld like any other change.
pub(crate) const GONE_BUDGET: u32 = 64;

/// Wall clock a single settled read or diff may take on the blocking pool -
/// and, separately, the wait for a read permit - before the delta is
/// abandoned as `Opaque(unreadable)`. A stalled network share or a frozen
/// filesystem must never park one of the two process-wide read permits for
/// the life of the watcher, nor park every other repo's worker behind them.
pub(crate) const READ_DEADLINE: Duration = Duration::from_secs(2);

/// Ceiling on `git show :0:./<path>` when fetching an index baseline.
pub(crate) const INDEX_BLOB_TIMEOUT: Duration = Duration::from_secs(5);

/// How long `stop` lets the worker observe the stop flag before aborting it;
/// one blocking read or diff at most, so a watcher restart never waits on a burst.
pub(crate) const STOP_GRACE: Duration = Duration::from_millis(250);

/// One repo's delta pipeline: the settle queue the watcher marks inline and the
/// worker task that resolves settled changes into `fileDelta` events. `note_*`
/// never block on IO and never await (ADR-009): lock, mutate, nudge.
pub(crate) struct DeltaService {
    queue: Arc<Mutex<SettleQueue>>,
    nudge: Arc<Notify>,
    stop: watch::Sender<bool>,
    /// Kills whatever `git show` the worker has in flight; cancelled before the
    /// join is even attempted, so `STOP_GRACE` is spent waiting on the loop
    /// rather than on a child process.
    cancel: BundleCancelToken,
    worker: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl DeltaService {
    pub(crate) fn new(
        repo_id: String,
        root: PathBuf,
        event_bus: EventBus,
        logger: Logger,
        tracked: TrackedPathSet,
        limits: DeltaLimits,
    ) -> Self {
        let queue = Arc::new(Mutex::new(SettleQueue::new()));
        let nudge = Arc::new(Notify::new());
        let (stop, stop_rx) = watch::channel(false);
        let cancel = BundleCancelToken::new();
        let links = delta_worker::WorkerLinks {
            queue: Arc::clone(&queue),
            nudge: Arc::clone(&nudge),
            stop: stop_rx,
            cancel: cancel.clone(),
        };
        let worker = delta_worker::DeltaWorker::new(
            repo_id, root, event_bus, logger, tracked, limits, links,
        );
        let handle = tokio::spawn(worker.run());
        Self {
            queue,
            nudge,
            stop,
            cancel,
            worker: Mutex::new(Some(handle)),
        }
    }

    /// A service with the queue but no worker task, so a test can assert on the
    /// marks the watcher left without a runner racing it to drain them.
    #[cfg(test)]
    pub(crate) fn new_queue_only() -> Self {
        Self {
            queue: Arc::new(Mutex::new(SettleQueue::new())),
            nudge: Arc::new(Notify::new()),
            stop: watch::channel(false).0,
            cancel: BundleCancelToken::new(),
            worker: Mutex::new(None),
        }
    }

    pub(crate) fn note_change(
        &self,
        path: String,
        abs_path: PathBuf,
        kind: FileKind,
        op: PendingOp,
    ) {
        self.lock_queue()
            .note(path, abs_path, kind, op, Instant::now());
        self.nudge.notify_one();
    }

    pub(crate) fn note_rename(&self, from: &str, to: &str, abs_to: PathBuf, kind: FileKind) {
        self.lock_queue()
            .note_rename(from, to, abs_to, kind, Instant::now());
        self.nudge.notify_one();
    }

    /// Signals stop, lets the worker observe it for `STOP_GRACE`, then aborts
    /// whatever is left. The worker owns the baseline cache, so it drops with it.
    pub(crate) async fn stop(&self) {
        let _ = self.stop.send(true);
        self.cancel.cancel();
        let handle = self
            .worker
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .take();
        if let Some(mut handle) = handle {
            if tokio::time::timeout(STOP_GRACE, &mut handle).await.is_err() {
                handle.abort();
            }
        }
    }

    /// Everything still pending, drained regardless of deadline.
    #[cfg(test)]
    pub(crate) fn take_pending(&self) -> Vec<PendingChange> {
        self.lock_queue()
            .drain_due(Instant::now() + MAX_LATENCY + MAX_LATENCY, usize::MAX)
    }

    fn lock_queue(&self) -> MutexGuard<'_, SettleQueue> {
        // Nothing awaits under this guard; recover a poisoned lock rather than
        // take the watcher down with it (ADR-008).
        self.queue.lock().unwrap_or_else(|err| err.into_inner())
    }
}
