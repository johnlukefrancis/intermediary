// Path: crates/im_agent/src/repos/delta/delta_worker.rs
// Description: The delta worker loop - drains settled changes, applies the burst budget, evicts stale baselines, stamps and publishes fileDelta and the counters it owes

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use im_bundle::cancel::BundleCancelToken;
use tokio::sync::{watch, Notify, Semaphore};

use crate::logging::Logger;
use crate::protocol::{AgentEvent, DeltaPayload, FileDeltaCountersEvent};
use crate::repos::source_control_watch::TrackedPathSet;
use crate::server::EventBus;

use super::delta_budget::{BurstBucket, Charge};
use super::delta_reads::{DiskReads, ReadSources};
use super::delta_resolve::{resolve, Resolution, ResolveContext};
use super::delta_worker_counters::DeltaCounters;
use super::delta_worker_emit::{file_delta_event, DeltaStamp};
use super::delta_worker_evict::{evict_withheld, DroppedEviction};
use super::{
    BaselineCache, DeltaLimits, PendingChange, SettleQueue, CACHE_BYTES_PER_REPO, DRAIN_BATCH,
};

/// The service side of the worker: the queue the watcher marks, the nudge
/// that wakes the loop, and the stop flag.
pub(super) struct WorkerLinks {
    pub(super) queue: Arc<Mutex<SettleQueue>>,
    pub(super) nudge: Arc<Notify>,
    pub(super) stop: watch::Receiver<bool>,
    pub(super) cancel: BundleCancelToken,
}

pub(super) struct DeltaWorker {
    repo_id: String,
    root: PathBuf,
    event_bus: EventBus,
    logger: Logger,
    tracked: TrackedPathSet,
    permits: Arc<Semaphore>,
    reads: Arc<dyn ReadSources>,
    queue: Arc<Mutex<SettleQueue>>,
    nudge: Arc<Notify>,
    stop: watch::Receiver<bool>,
    cancel: BundleCancelToken,
    cache: BaselineCache,
    burst: BurstBucket,
    counters: DeltaCounters,
    dropped_eviction: DroppedEviction,
}

impl DeltaWorker {
    pub(super) fn new(
        repo_id: String,
        root: PathBuf,
        event_bus: EventBus,
        logger: Logger,
        tracked: TrackedPathSet,
        limits: DeltaLimits,
        links: WorkerLinks,
    ) -> Self {
        Self {
            repo_id,
            root,
            event_bus,
            logger,
            tracked,
            permits: limits.read_permits,
            reads: Arc::new(DiskReads),
            queue: links.queue,
            nudge: links.nudge,
            stop: links.stop,
            cancel: links.cancel,
            cache: BaselineCache::new(CACHE_BYTES_PER_REPO),
            burst: BurstBucket::new(Instant::now()),
            counters: DeltaCounters::new(Instant::now()),
            dropped_eviction: DroppedEviction::new(),
        }
    }

    /// `select!` over stop, the earliest settle deadline (armed only while a
    /// change is pending, so an idle worker holds no timer) and the nudge.
    pub(super) async fn run(mut self) {
        let timer = tokio::time::sleep(Duration::ZERO);
        tokio::pin!(timer);
        loop {
            let deadline = self.lock_queue().next_deadline();
            let armed = deadline.is_some();
            if let Some(deadline) = deadline {
                timer
                    .as_mut()
                    .reset(tokio::time::Instant::from_std(deadline));
            }
            tokio::select! {
                _ = self.stop.changed() => break,
                _ = &mut timer, if armed => {}
                _ = self.nudge.notified() => {}
            }
            if *self.stop.borrow() {
                break;
            }

            let now = Instant::now();
            // The refill is judged on the queue as it stands BEFORE this drain,
            // batch included: mid-run the count stays above the refill ceiling.
            let (pending, batch, dropped, dropped_paths, overflowed) = {
                let mut queue = self.queue.lock().unwrap_or_else(|err| err.into_inner());
                let pending = queue.len();
                let batch = queue.drain_due(now, DRAIN_BATCH);
                let (dropped, dropped_paths, overflowed) = queue.take_dropped();
                (pending, batch, dropped, dropped_paths, overflowed)
            };
            let window_denied = self.burst.roll(now, pending, &self.logger, &self.repo_id);
            self.counters.note_dropped(dropped);
            self.dropped_eviction.apply(
                &mut self.cache,
                &dropped_paths,
                overflowed,
                &self.logger,
                &self.repo_id,
            );
            if !self.event_bus.has_receivers() {
                // Idle daemon: nobody is listening, so a sighting only evicts
                // the baseline; the next sighting after a subscriber arrives
                // says `VS INDEX`.
                for change in &batch {
                    self.cache.remove(&change.path);
                }
                continue;
            }
            for change in batch {
                if *self.stop.borrow() {
                    return;
                }
                self.process(change).await;
            }
            let quiet = self.lock_queue().next_deadline().is_none();
            if self
                .counters
                .standalone_due(Instant::now(), window_denied, quiet)
            {
                self.flush_counters();
            }
        }
    }

    /// Publishes the counters on their own (`DeltaCounters::standalone_due`
    /// says when). Whichever carrier goes first delivers, and both take the
    /// counters, so the UI never sees the same withheld path twice.
    pub(super) fn flush_counters(&mut self) {
        if self.counters.is_zero() {
            return;
        }
        let taken = self.counters.take(Instant::now());
        self.event_bus
            .broadcast_event(AgentEvent::FileDeltaCounters(FileDeltaCountersEvent {
                repo_id: self.repo_id.clone(),
                seq: taken.seq,
                withheld: taken.withheld,
                dropped: taken.dropped,
            }));
    }

    pub(super) async fn process(&mut self, mut change: PendingChange) {
        // A re-settled change already paid its token on the first attempt;
        // charging again would let one stubborn file eat the whole window.
        let charge = if change.resettles > 0 {
            Charge::Resolve
        } else {
            self.burst.charge(&change.op)
        };
        let may_spawn = match charge {
            Charge::Resolve => true,
            Charge::Withhold => {
                self.counters.note_withheld();
                evict_withheld(&mut self.cache, &change);
                return;
            }
            // The delete still prints, as a `Gone` card with no baseline.
            Charge::GoneOnly => false,
        };
        let mut context = ResolveContext {
            root: &self.root,
            cache: &mut self.cache,
            permits: &self.permits,
            reads: &self.reads,
            cancel: &self.cancel,
            may_spawn,
        };
        match resolve(&mut context, &mut change).await {
            Resolution::Resettle => self.lock_queue().requeue(change, Instant::now()),
            Resolution::Drop => {}
            Resolution::Emit {
                payload,
                mtime,
                failure,
            } => {
                if let Some(reason) = failure {
                    if self.burst.first_failure(&change.path) {
                        self.logger.warn(
                            "Delta read failed",
                            Some(serde_json::json!({
                                "repoId": self.repo_id,
                                "path": change.path,
                                "reason": reason,
                            })),
                        );
                    }
                }
                self.emit(change, payload, mtime);
            }
        }
    }

    fn emit(&mut self, change: PendingChange, payload: DeltaPayload, mtime: String) {
        let taken = self.counters.take(Instant::now());
        let stamp = DeltaStamp {
            repo_id: self.repo_id.clone(),
            seq: taken.seq,
            tracked: self.tracked.contains(&change.path),
            withheld: taken.withheld,
            dropped: taken.dropped,
        };
        let event = file_delta_event(stamp, change, payload, mtime);
        self.logger.debug(
            "fileDelta",
            Some(serde_json::json!({
                "repoId": event.repo_id,
                "seq": event.seq,
                "path": event.path,
                "op": event.op,
                "folded": event.folded,
                "withheld": event.withheld,
                "dropped": event.dropped,
                "size": event.payload_size(),
                "cacheBytes": self.cache.bytes(),
            })),
        );
        self.event_bus.broadcast_event(AgentEvent::FileDelta(event));
    }

    fn lock_queue(&self) -> std::sync::MutexGuard<'_, SettleQueue> {
        self.queue.lock().unwrap_or_else(|err| err.into_inner())
    }
}
