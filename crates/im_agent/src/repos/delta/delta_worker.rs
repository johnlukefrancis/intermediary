// Path: crates/im_agent/src/repos/delta/delta_worker.rs
// Description: The delta worker loop - drains settled changes, applies the burst budget, stamps and publishes fileDelta

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use im_bundle::cancel::BundleCancelToken;
use tokio::sync::{watch, Notify, Semaphore};

use crate::logging::Logger;
use crate::protocol::{AgentEvent, DeltaOp, DeltaPayload, FileDeltaCountersEvent, FileDeltaEvent};
use crate::repos::source_control_watch::TrackedPathSet;
use crate::server::EventBus;

use super::delta_budget::{BurstBucket, Charge};
use super::delta_resolve::{resolve, Resolution, ResolveContext};
use super::{
    BaselineCache, DeltaLimits, PendingChange, PendingOp, SettleQueue, CACHE_BYTES_PER_REPO,
    DRAIN_BATCH,
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
    queue: Arc<Mutex<SettleQueue>>,
    nudge: Arc<Notify>,
    stop: watch::Receiver<bool>,
    cancel: BundleCancelToken,
    cache: BaselineCache,
    burst: BurstBucket,
    seq: u64,
    withheld: u32,
    dropped: u32,
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
            queue: links.queue,
            nudge: links.nudge,
            stop: links.stop,
            cancel: links.cancel,
            cache: BaselineCache::new(CACHE_BYTES_PER_REPO),
            burst: BurstBucket::new(Instant::now()),
            seq: 0,
            withheld: 0,
            dropped: 0,
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
            let window_denied = self.burst.roll(now, &self.logger, &self.repo_id);
            let (batch, dropped, dropped_paths) = {
                let mut queue = self.lock_queue();
                let batch = queue.drain_due(now, DRAIN_BATCH);
                let (dropped, dropped_paths) = queue.take_dropped();
                (batch, dropped, dropped_paths)
            };
            self.dropped = self.dropped.saturating_add(dropped);
            // A mark discarded at `QUEUE_CAP` never reached the resolver, so the
            // cached text is no longer what the reader last saw.
            for path in &dropped_paths {
                self.cache.remove(path);
            }
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
            if window_denied || self.lock_queue().next_deadline().is_none() {
                self.flush_counters();
            }
        }
    }

    /// Publishes the counters on their own when nothing is left to piggyback
    /// them on: the queue went quiet, or a burst window that denied something
    /// closed. Whichever carrier goes first delivers, and both take the
    /// counters, so the UI never sees the same withheld path twice.
    fn flush_counters(&mut self) {
        if self.withheld == 0 && self.dropped == 0 {
            return;
        }
        self.event_bus
            .broadcast_event(AgentEvent::FileDeltaCounters(FileDeltaCountersEvent {
                repo_id: self.repo_id.clone(),
                withheld: std::mem::take(&mut self.withheld),
                dropped: std::mem::take(&mut self.dropped),
            }));
    }

    async fn process(&mut self, change: PendingChange) {
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
                self.withheld = self.withheld.saturating_add(1);
                self.cache.remove(&change.path);
                return;
            }
            // The delete still prints, as a `Gone` card with no baseline.
            Charge::GoneOnly => false,
        };
        let mut context = ResolveContext {
            root: &self.root,
            cache: &mut self.cache,
            permits: &self.permits,
            cancel: &self.cancel,
            may_spawn,
        };
        match resolve(&mut context, &change).await {
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
        self.seq = self.seq.saturating_add(1);
        let (op, from_path) = match change.op {
            PendingOp::Add => (DeltaOp::Add, None),
            PendingOp::Modify => (DeltaOp::Modify, None),
            PendingOp::Remove => (DeltaOp::Remove, None),
            PendingOp::Rename { from } => (DeltaOp::Rename, Some(from)),
        };
        let size = match &payload {
            DeltaPayload::Text { patch, .. } => patch.len() as u64,
            DeltaPayload::Image { bytes, .. } | DeltaPayload::Opaque { bytes, .. } => *bytes,
            DeltaPayload::Gone => 0,
        };
        let event = FileDeltaEvent {
            repo_id: self.repo_id.clone(),
            seq: self.seq,
            tracked: Some(self.tracked.contains(&change.path)),
            path: change.path,
            from_path,
            kind: change.kind,
            op,
            mtime,
            folded: change.folded,
            withheld: std::mem::take(&mut self.withheld),
            dropped: std::mem::take(&mut self.dropped),
            payload,
        };
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
                "size": size,
                "cacheBytes": self.cache.bytes(),
            })),
        );
        self.event_bus.broadcast_event(AgentEvent::FileDelta(event));
    }

    fn lock_queue(&self) -> std::sync::MutexGuard<'_, SettleQueue> {
        self.queue.lock().unwrap_or_else(|err| err.into_inner())
    }
}
