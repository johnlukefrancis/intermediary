// Path: crates/im_agent/src/server/event_bus_relay.rs
// Description: The per-connection relay - a drop-on-full stream lane under slot and byte ceilings, a backpressured control lane, and the biased reader the socket writer drains

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use serde_json::json;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;

use crate::logging::Logger;

use super::{
    BusMessage, EventLane, CONTROL_QUEUE_CAP, EVENT_DROP_LOG_INTERVAL, EVENT_QUEUE_BYTES,
    EVENT_QUEUE_CAP,
};

/// The reading end of one connection's two lanes. `recv` prefers the control
/// lane, so a snapshot or an error is never queued behind a burst of deltas.
pub struct EventRelay {
    pub(super) control: mpsc::Receiver<String>,
    pub(super) stream: mpsc::Receiver<String>,
    pub(super) queued: Arc<QueuedBytes>,
}

impl EventRelay {
    /// The next event for the writer, control lane first. `None` once the
    /// relay task is gone and both lanes are drained.
    pub async fn recv(&mut self) -> Option<String> {
        let next = tokio::select! {
            biased;
            control = self.control.recv() => control,
            stream = self.stream.recv() => stream,
        };
        // Both senders live on the one relay task, so a closed control lane
        // means a closed stream lane too: hand over whatever it still holds.
        let text = match next {
            Some(text) => text,
            None => self.stream.recv().await?,
        };
        self.queued.release(text.len());
        Some(text)
    }
}

/// Bytes queued across both lanes of one connection, against one ceiling.
/// Added by the relay task, released by the writer as it takes each event.
pub(super) struct QueuedBytes {
    bytes: AtomicUsize,
    ceiling: usize,
}

impl QueuedBytes {
    pub(super) fn new(ceiling: usize) -> Self {
        Self {
            bytes: AtomicUsize::new(0),
            ceiling,
        }
    }

    /// Reserves `len` bytes, or refuses when the ceiling would be crossed.
    pub(super) fn admit(&self, len: usize) -> bool {
        let prior = self.bytes.fetch_add(len, Ordering::AcqRel);
        if prior.saturating_add(len) > self.ceiling {
            self.release(len);
            return false;
        }
        true
    }

    /// Reserves `len` bytes unconditionally: the control lane's own cap is
    /// what bounds it, but its bytes still count toward the stream lane's gate.
    pub(super) fn charge(&self, len: usize) {
        self.bytes.fetch_add(len, Ordering::AcqRel);
    }

    pub(super) fn release(&self, len: usize) {
        self.bytes.fetch_sub(len, Ordering::AcqRel);
    }

    #[cfg(test)]
    pub(super) fn queued(&self) -> usize {
        self.bytes.load(Ordering::Acquire)
    }
}

/// Forwards one subscription into the two lanes. A stream event is `try_send`:
/// a full lane or a crossed byte ceiling drops it, counts it, and warns at
/// most once per `EVENT_DROP_LOG_INTERVAL`. A control event is awaited: when
/// its lane fills the relay stops taking from the broadcast, which then lags
/// (the pre-existing behaviour) rather than the event being silently lost.
pub(super) fn spawn(
    mut broadcast_rx: broadcast::Receiver<BusMessage>,
    logger: Logger,
    peer: String,
    lagged_message: &'static str,
) -> (EventRelay, JoinHandle<()>) {
    let (control_tx, control) = mpsc::channel::<String>(CONTROL_QUEUE_CAP);
    let (stream_tx, stream) = mpsc::channel::<String>(EVENT_QUEUE_CAP);
    let queued = Arc::new(QueuedBytes::new(EVENT_QUEUE_BYTES));
    let budget = Arc::clone(&queued);
    let task = tokio::spawn(async move {
        let mut drops = DropGate::new();
        loop {
            let message = match broadcast_rx.recv().await {
                Ok(message) => message,
                Err(broadcast::error::RecvError::Closed) => break,
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    logger.warn(lagged_message, Some(json!({"skipped": skipped})));
                    continue;
                }
            };
            let len = message.text.len();
            match message.lane {
                EventLane::Control => {
                    budget.charge(len);
                    if control_tx.send(message.text).await.is_err() {
                        break;
                    }
                }
                EventLane::Stream => {
                    if !budget.admit(len) {
                        warn_drop(&mut drops, &logger, &peer);
                        continue;
                    }
                    match stream_tx.try_send(message.text) {
                        Ok(()) => {}
                        Err(mpsc::error::TrySendError::Full(_)) => {
                            budget.release(len);
                            warn_drop(&mut drops, &logger, &peer);
                        }
                        Err(mpsc::error::TrySendError::Closed(_)) => break,
                    }
                }
            }
        }
    });
    (
        EventRelay {
            control,
            stream,
            queued,
        },
        task,
    )
}

fn warn_drop(drops: &mut DropGate, logger: &Logger, peer: &str) {
    if let Some(count) = drops.note(Instant::now()) {
        logger.warn(
            "Event queue full; events dropped",
            Some(json!({"peer": peer, "dropped": count})),
        );
    }
}

/// Counts dropped events and says when a warn is due: the first drop of a
/// burst logs at once, later ones fold into one line per interval.
pub(super) struct DropGate {
    pending: u64,
    last_logged: Option<Instant>,
}

impl DropGate {
    pub(super) fn new() -> Self {
        Self {
            pending: 0,
            last_logged: None,
        }
    }

    /// Records one drop; `Some(count)` when a warn is due, carrying every drop
    /// since the previous warn.
    pub(super) fn note(&mut self, now: Instant) -> Option<u64> {
        self.pending = self.pending.saturating_add(1);
        let due = self
            .last_logged
            .is_none_or(|last| now.saturating_duration_since(last) >= EVENT_DROP_LOG_INTERVAL);
        if !due {
            return None;
        }
        self.last_logged = Some(now);
        Some(std::mem::take(&mut self.pending))
    }
}
