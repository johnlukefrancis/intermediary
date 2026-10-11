// Path: crates/im_host_agent/src/wsl/wsl_transport_status_tests.rs
// Description: WSL transport status tests: offline error de-noising and online recovery per generation

use std::time::Duration;

use im_agent::protocol::EventEnvelope;
use im_agent::server::BusMessage;
use tokio::sync::broadcast::Receiver;

use super::*;

pub(crate) async fn recv_payload(rx: &mut Receiver<BusMessage>) -> AgentEvent {
    let wire = rx.recv().await.expect("event payload");
    let envelope: EventEnvelope = serde_json::from_str(&wire).expect("event envelope");
    envelope.payload
}

pub(crate) async fn assert_no_event(rx: &mut Receiver<BusMessage>) {
    let result = tokio::time::timeout(Duration::from_millis(25), rx.recv()).await;
    assert!(result.is_err(), "unexpected extra event");
}

fn assert_transport_error(event: AgentEvent, expected_raw_code: &str) {
    match event {
        AgentEvent::Error(payload) => {
            let details = payload.details.expect("error details");
            assert_eq!(details.raw_code.as_deref(), Some(expected_raw_code));
        }
        _ => panic!("expected error event"),
    }
}

pub(crate) fn assert_online_status(event: AgentEvent, expected_generation: u64) {
    match event {
        AgentEvent::WslBackendStatus(payload) => {
            assert_eq!(payload.status, WslBackendConnectionStatus::Online);
            assert_eq!(payload.generation, expected_generation);
        }
        _ => panic!("expected wslBackendStatus event"),
    }
}

fn status_for_test() -> (WslTransportStatus, Receiver<BusMessage>) {
    let event_bus = EventBus::new(16);
    let rx = event_bus.subscribe();
    (
        WslTransportStatus::new(event_bus, Arc::new(AtomicU64::new(0))),
        rx,
    )
}

#[tokio::test]
async fn emits_offline_transport_error_once_per_generation() {
    let (status, mut rx) = status_for_test();
    let err = AgentError::new(WSL_BACKEND_UNAVAILABLE, "WSL backend unavailable");

    status.report_transport_error_for_generation(&err, None, 3);
    status.report_transport_error_for_generation(&err, None, 3);
    assert_transport_error(recv_payload(&mut rx).await, WSL_BACKEND_UNAVAILABLE);
    assert_no_event(&mut rx).await;

    status.report_transport_error_for_generation(&err, None, 4);
    assert_transport_error(recv_payload(&mut rx).await, WSL_BACKEND_UNAVAILABLE);
    assert_no_event(&mut rx).await;
}

#[tokio::test]
async fn non_transport_errors_are_not_reported_as_offline() {
    let (status, mut rx) = status_for_test();
    let err = AgentError::new("GIT_FAILED", "git failed");
    status.report_transport_error_for_generation(&err, None, 1);
    assert_no_event(&mut rx).await;
}

#[tokio::test]
async fn emits_online_recovery_once_on_first_success_after_offline_error() {
    let (status, mut rx) = status_for_test();
    let err = AgentError::new(WSL_BACKEND_TIMEOUT, "WSL backend timed out");

    status.mark_success(7);
    assert_no_event(&mut rx).await;

    status.report_transport_error_for_generation(&err, None, 7);
    assert_transport_error(recv_payload(&mut rx).await, WSL_BACKEND_TIMEOUT);
    status.mark_success(7);
    assert_online_status(recv_payload(&mut rx).await, 7);
    status.mark_success(7);
    assert_no_event(&mut rx).await;

    status.report_transport_error_for_generation(&err, None, 8);
    assert_transport_error(recv_payload(&mut rx).await, WSL_BACKEND_TIMEOUT);
    status.mark_success(9);
    assert_no_event(&mut rx).await;
}

#[tokio::test]
async fn announced_online_clears_the_offline_error_for_its_generation() {
    let (status, mut rx) = status_for_test();
    let err = AgentError::new(WSL_BACKEND_TIMEOUT, "WSL backend timed out");

    status.report_transport_error_for_generation(&err, None, 2);
    assert_transport_error(recv_payload(&mut rx).await, WSL_BACKEND_TIMEOUT);
    status.announce_online(2);
    assert_online_status(recv_payload(&mut rx).await, 2);
    status.mark_success(2);
    assert_no_event(&mut rx).await;

    status.report_transport_error_for_generation(&err, None, 2);
    assert_transport_error(recv_payload(&mut rx).await, WSL_BACKEND_TIMEOUT);
}
