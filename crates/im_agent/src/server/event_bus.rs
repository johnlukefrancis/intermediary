// Path: crates/im_agent/src/server/event_bus.rs
// Description: Broadcast agent events to connected WebSocket clients - each event serialized once and tagged with its lane at broadcast time

use std::fmt;
use std::ops::Deref;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::broadcast;
use tokio::task::JoinHandle;

use crate::logging::Logger;
use crate::protocol::{AgentEvent, EventEnvelope};

#[path = "event_bus_relay.rs"]
mod relay;

pub use relay::EventRelay;

/// Stream events queued for one socket writer: the lane that is dropped on
/// full. Bounded so a stalled client can never grow agent memory: a full lane
/// drops the event (the UI sees the `seq` gap) instead of parking the relay.
pub(crate) const EVENT_QUEUE_CAP: usize = 1024;

/// Bytes queued for one socket writer across BOTH lanes. `EVENT_QUEUE_CAP`
/// slots of 64 KiB patches would be 64 MiB per stalled client; this ceiling
/// drops a stream event first, so a connection holds at most 8 MiB.
pub(crate) const EVENT_QUEUE_BYTES: usize = 8 * 1024 * 1024;

/// Control events queued for one socket writer: snapshots, topology, source
/// control, bundles, errors, backend status. Never dropped - the relay awaits
/// this lane, letting the broadcast receiver lag instead - and small, because
/// a control event the writer cannot take within 64 slots is a dead client.
pub(crate) const CONTROL_QUEUE_CAP: usize = 64;

/// One warn per connection per this interval while events are being dropped,
/// carrying the count since the previous line.
pub(crate) const EVENT_DROP_LOG_INTERVAL: Duration = Duration::from_secs(5);

/// Which per-connection lane an event rides. Decided from the variant when
/// the event is broadcast, so no relay ever re-parses the wire text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventLane {
    /// `fileChanged`, `fileDelta`, `fileDeltaCounters`: high-volume, and the
    /// shared `seq` makes a loss visible, so a full lane drops.
    Stream,
    /// Everything else: state a client cannot recover from a gap in, so the
    /// lane applies backpressure instead of dropping.
    Control,
}

/// One serialized event as every subscriber sees it: the wire text, plus the
/// lane it belongs to. Reads as its text.
#[derive(Debug, Clone)]
pub struct BusMessage {
    pub lane: EventLane,
    pub text: String,
}

impl Deref for BusMessage {
    type Target = str;

    fn deref(&self) -> &str {
        &self.text
    }
}

impl fmt::Display for BusMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

#[derive(Clone)]
pub struct EventBus {
    inner: Arc<EventBusInner>,
}

struct EventBusInner {
    sender: broadcast::Sender<BusMessage>,
    counter: AtomicU64,
}

impl EventBus {
    pub fn new(buffer: usize) -> Self {
        let (sender, _) = broadcast::channel(buffer);
        Self {
            inner: Arc::new(EventBusInner {
                sender,
                counter: AtomicU64::new(0),
            }),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<BusMessage> {
        self.inner.sender.subscribe()
    }

    /// Whether anyone is subscribed; producers with real cost (delta reads)
    /// skip their work for an idle daemon.
    pub fn has_receivers(&self) -> bool {
        self.inner.sender.receiver_count() > 0
    }

    pub fn broadcast_event(&self, event: AgentEvent) {
        let lane = lane_of(&event);
        let event_id = self.next_event_id();
        let envelope = EventEnvelope {
            kind: "event".to_string(),
            event_id: Some(event_id),
            payload: event,
        };

        if let Ok(text) = serde_json::to_string(&envelope) {
            let _ = self.inner.sender.send(BusMessage { lane, text });
        }
    }

    /// Subscribes on behalf of one connection and forwards into two bounded
    /// lanes (`event_bus_relay.rs`): stream events through `EVENT_QUEUE_CAP`
    /// slots / `EVENT_QUEUE_BYTES` with drop-on-full, control events through
    /// `CONTROL_QUEUE_CAP` with backpressure. `lagged_message` is the
    /// connection's own broadcast-lag line. Abort the task when the connection
    /// ends; the lanes then drain and close.
    pub fn relay(
        &self,
        logger: Logger,
        peer: String,
        lagged_message: &'static str,
    ) -> (EventRelay, JoinHandle<()>) {
        relay::spawn(self.subscribe(), logger, peer, lagged_message)
    }

    fn next_event_id(&self) -> String {
        let next = self.inner.counter.fetch_add(1, Ordering::Relaxed) + 1;
        format!("evt_{next}")
    }
}

/// The lane table. Exhaustive on purpose: a new event kind has to say which
/// lane it rides before it can be broadcast.
fn lane_of(event: &AgentEvent) -> EventLane {
    match event {
        AgentEvent::FileChanged(_)
        | AgentEvent::FileDelta(_)
        | AgentEvent::FileDeltaCounters(_) => EventLane::Stream,
        AgentEvent::Snapshot(_)
        | AgentEvent::RepoTopologyChanged(_)
        | AgentEvent::BundleBuilt(_)
        | AgentEvent::BundleBuildProgress(_)
        | AgentEvent::Error(_)
        | AgentEvent::WslBackendStatus(_)
        | AgentEvent::SourceControlChanged(_) => EventLane::Control,
    }
}

#[cfg(test)]
#[path = "event_bus_tests.rs"]
mod tests;
