// Path: crates/im_agent/src/server/event_bus_tests.rs
// Description: Event bus tests - lane tagging at broadcast, a stream flood against a stalled writer, the byte ceiling gate, the drop-warn gate

use std::time::Instant;

use tempfile::tempdir;

use crate::logging::{LogConfig, LogLevel, Logger};
use crate::protocol::{AgentErrorEvent, AgentEvent, FileDeltaCountersEvent};

use super::relay::{DropGate, QueuedBytes};
use super::{EventBus, EventLane, EVENT_DROP_LOG_INTERVAL, EVENT_QUEUE_CAP};

fn control(message: &str) -> AgentEvent {
    AgentEvent::Error(AgentErrorEvent::new("test", message, None))
}

fn stream(seq: u64) -> AgentEvent {
    AgentEvent::FileDeltaCounters(FileDeltaCountersEvent {
        repo_id: "repo-1".to_string(),
        seq,
        withheld: 1,
        dropped: 0,
    })
}

async fn logger() -> (tempfile::TempDir, Logger) {
    let temp = tempdir().expect("tempdir");
    let logger = Logger::init(LogConfig {
        log_dir: temp.path().join("logs"),
        min_level: LogLevel::Error,
        emit_stdio: false,
    })
    .await
    .expect("logger");
    (temp, logger)
}

/// The lane is decided from the variant when the event is broadcast and
/// travels with the wire text, so no relay has to look inside the JSON.
#[tokio::test]
async fn the_lane_is_tagged_at_broadcast() {
    let bus = EventBus::new(8);
    let mut events = bus.subscribe();
    bus.broadcast_event(control("hello"));
    bus.broadcast_event(stream(1));

    let first = events.try_recv().expect("the control event");
    assert_eq!(first.lane, EventLane::Control);
    assert!(first.contains("\"type\":\"error\""), "{}", first.text);
    let second = events.try_recv().expect("the stream event");
    assert_eq!(second.lane, EventLane::Stream);
    assert!(
        second.contains("\"type\":\"fileDeltaCounters\""),
        "{}",
        second.text
    );
}

/// With the writer stalled, a flood past `EVENT_QUEUE_CAP` drops stream
/// events - and only stream events: every control event broadcast before,
/// during and after the flood arrives, in order.
#[tokio::test]
async fn a_stream_flood_against_a_stalled_writer_never_drops_a_control_event() {
    let (_temp, logger) = logger().await;
    let flood = EVENT_QUEUE_CAP + 200;
    let bus = EventBus::new(flood + 8);
    let (mut relay, task) = bus.relay(logger, "peer".to_string(), "lagged");

    bus.broadcast_event(control("before"));
    for seq in 0..flood {
        bus.broadcast_event(stream(seq as u64));
    }
    bus.broadcast_event(control("after"));
    bus.broadcast_event(control("last"));

    // Nothing is taken from the stream lane while the flood lands.
    let mut control_bytes = 0_usize;
    for expected in ["before", "after", "last"] {
        let text = relay.control.recv().await.expect("a control event");
        assert!(text.contains(expected), "expected {expected}, got {text}");
        control_bytes += text.len();
    }

    task.abort();
    let _ = task.await;
    let mut streamed = 0_usize;
    while relay.recv().await.is_some() {
        streamed += 1;
    }
    assert_eq!(streamed, EVENT_QUEUE_CAP, "the lane held exactly its cap");
    assert_eq!(
        relay.queued.queued(),
        control_bytes,
        "every stream byte the writer took was released; the lane-read controls were not"
    );
}

/// The byte ceiling admits up to the line and refuses past it, and a release
/// makes room again.
#[test]
fn the_byte_ceiling_admits_exactly_to_the_line() {
    let queued = QueuedBytes::new(100);
    assert!(queued.admit(60));
    assert!(!queued.admit(50), "110 would cross the ceiling");
    assert_eq!(queued.queued(), 60, "a refused event holds no bytes");
    assert!(queued.admit(40), "exactly at the ceiling is admitted");
    queued.release(60);
    assert!(queued.admit(50));
    assert_eq!(queued.queued(), 90);
}

#[test]
fn drop_gate_warns_once_per_interval_with_the_count() {
    let start = Instant::now();
    let mut gate = DropGate::new();
    assert_eq!(gate.note(start), Some(1), "the first drop warns at once");
    assert_eq!(gate.note(start + EVENT_DROP_LOG_INTERVAL / 2), None);
    assert_eq!(gate.note(start + EVENT_DROP_LOG_INTERVAL / 2), None);
    assert_eq!(
        gate.note(start + EVENT_DROP_LOG_INTERVAL),
        Some(3),
        "the interval's drops fold into one line"
    );
    assert_eq!(gate.note(start + EVENT_DROP_LOG_INTERVAL), None);
}
