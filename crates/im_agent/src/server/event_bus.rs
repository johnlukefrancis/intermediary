// Path: crates/im_agent/src/server/event_bus.rs
// Description: Broadcast agent events to connected WebSocket clients

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio::sync::broadcast;

use crate::protocol::{AgentEvent, EventEnvelope};

#[derive(Clone)]
pub struct EventBus {
    inner: Arc<EventBusInner>,
}

struct EventBusInner {
    sender: broadcast::Sender<String>,
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

    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.inner.sender.subscribe()
    }

    pub fn broadcast_event(&self, event: AgentEvent) {
        let event_id = self.next_event_id();
        let envelope = EventEnvelope {
            kind: "event".to_string(),
            event_id: Some(event_id),
            payload: event,
        };

        if let Ok(text) = serde_json::to_string(&envelope) {
            let _ = self.inner.sender.send(text);
        }
    }

    fn next_event_id(&self) -> String {
        let next = self.inner.counter.fetch_add(1, Ordering::Relaxed) + 1;
        format!("evt_{next}")
    }
}
