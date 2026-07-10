// Path: crates/im_host_agent/src/runtime/host_runtime/wsl_routing_tests.rs
// Description: WSL transport transition tests for host runtime routing

use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use im_agent::logging::{LogConfig, LogLevel, Logger};
use im_agent::protocol::{AgentEvent, EventEnvelope};

use super::*;

async fn host_runtime_for_test() -> HostRuntime {
    let logger = Logger::init(LogConfig {
        log_dir: unique_log_dir(),
        min_level: LogLevel::Error,
        emit_stdio: false,
    })
    .await
    .expect("logger init");
    HostRuntime::new(0, "test_token".to_string(), logger)
}

fn unique_log_dir() -> PathBuf {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock");
    std::env::temp_dir().join(format!("im_host_agent_wsl_routing_{}", now.as_nanos()))
}

async fn recv_payload(rx: &mut tokio::sync::broadcast::Receiver<String>) -> AgentEvent {
    let wire = rx.recv().await.expect("event payload");
    let envelope: EventEnvelope = serde_json::from_str(&wire).expect("event envelope");
    envelope.payload
}

async fn assert_no_event(rx: &mut tokio::sync::broadcast::Receiver<String>) {
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

fn assert_online_status(event: AgentEvent, expected_generation: u64) {
    match event {
        AgentEvent::WslBackendStatus(payload) => {
            assert_eq!(payload.status, WslBackendConnectionStatus::Online);
            assert_eq!(payload.generation, expected_generation);
        }
        _ => panic!("expected wslBackendStatus event"),
    }
}

#[tokio::test]
async fn emits_offline_transport_error_once_per_generation() {
    let mut runtime = host_runtime_for_test().await;
    let event_bus = EventBus::new(16);
    let mut rx = event_bus.subscribe();
    let err = AgentError::new(WSL_BACKEND_UNAVAILABLE, "WSL backend unavailable");

    runtime.emit_wsl_unavailable_if_transport_error_for_generation(&err, &event_bus, None, 3);
    runtime.emit_wsl_unavailable_if_transport_error_for_generation(&err, &event_bus, None, 3);

    assert_transport_error(recv_payload(&mut rx).await, WSL_BACKEND_UNAVAILABLE);
    assert_no_event(&mut rx).await;

    runtime.emit_wsl_unavailable_if_transport_error_for_generation(&err, &event_bus, None, 4);
    assert_transport_error(recv_payload(&mut rx).await, WSL_BACKEND_UNAVAILABLE);
    assert_no_event(&mut rx).await;
}

#[tokio::test]
async fn emits_online_recovery_once_on_first_success_after_offline_error() {
    let mut runtime = host_runtime_for_test().await;
    let event_bus = EventBus::new(16);
    let mut rx = event_bus.subscribe();
    let err = AgentError::new(WSL_BACKEND_TIMEOUT, "WSL backend timed out");

    runtime.mark_wsl_transport_success_for_generation(&event_bus, 7);
    assert_no_event(&mut rx).await;

    runtime.emit_wsl_unavailable_if_transport_error_for_generation(&err, &event_bus, None, 7);
    assert_transport_error(recv_payload(&mut rx).await, WSL_BACKEND_TIMEOUT);

    runtime.mark_wsl_transport_success_for_generation(&event_bus, 7);
    assert_online_status(recv_payload(&mut rx).await, 7);

    runtime.mark_wsl_transport_success_for_generation(&event_bus, 7);
    assert_no_event(&mut rx).await;

    runtime.emit_wsl_unavailable_if_transport_error_for_generation(&err, &event_bus, None, 8);
    assert_transport_error(recv_payload(&mut rx).await, WSL_BACKEND_TIMEOUT);

    runtime.mark_wsl_transport_success_for_generation(&event_bus, 8);
    assert_online_status(recv_payload(&mut rx).await, 8);
    assert_no_event(&mut rx).await;
}
