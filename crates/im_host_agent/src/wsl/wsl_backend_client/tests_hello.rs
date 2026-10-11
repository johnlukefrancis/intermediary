// Path: crates/im_host_agent/src/wsl/wsl_backend_client/tests_hello.rs
// Description: Hello reconciler tests: online once per generation, failure hand-off, root identity version gate

use serde_json::json;
use tokio::sync::broadcast::Receiver;

use super::hello_reconciler::require_root_identity_version;
use super::tests::quiet_logger;
use super::*;
use crate::error_codes::WSL_BACKEND_VERSION_MISMATCH;
use crate::wsl::wsl_transport_status::tests::{
    assert_no_event, assert_online_status, recv_payload,
};
use im_agent::protocol::{ClientHelloCommand, ClientHelloResult};
use im_agent::server::BusMessage;

const PROMPT: Duration = Duration::from_secs(2);

fn wsl_hello() -> ClientHelloCommand {
    ClientHelloCommand {
        config: json!({"repos": [{"repoId": "wsl_repo", "root": {"kind": "wsl", "path": "/repo"}}]}),
        staging_host_root: "C:\\staging".to_string(),
        staging_wsl_root: Some("/mnt/c/staging".to_string()),
        auto_stage_on_change: Some(false),
    }
}

async fn next_forward(rx: &mut mpsc::UnboundedReceiver<RequestLoopMessage>) -> Box<ForwardRequest> {
    loop {
        match timeout(PROMPT, rx.recv())
            .await
            .expect("forward")
            .expect("loop open")
        {
            RequestLoopMessage::Forward(request) => return request,
            RequestLoopMessage::Cancel { .. } => {}
        }
    }
}

async fn assert_no_forward(rx: &mut mpsc::UnboundedReceiver<RequestLoopMessage>) {
    let next = timeout(Duration::from_millis(50), rx.recv()).await;
    assert!(next.is_err(), "unexpected extra WSL request");
}

fn applied(request: Box<ForwardRequest>, generation: u64) {
    assert!(matches!(request.command, UiCommand::ClientHello(_)));
    let response = UiResponse::ClientHelloResult(ClientHelloResult {
        agent_version: "0.1.24".to_string(),
        watched_repo_ids: vec!["wsl_repo".to_string()],
    });
    let _ = request.response_tx.send(Ok(ForwardedWslResponse {
        response,
        generation,
    }));
}

async fn scripted() -> (
    WslBackendClient,
    mpsc::UnboundedReceiver<RequestLoopMessage>,
    Receiver<BusMessage>,
) {
    let event_bus = EventBus::new(32);
    let events = event_bus.subscribe();
    let (client, rx) = WslBackendClient::with_request_channel(event_bus, quiet_logger().await);
    (client, rx, events)
}

#[tokio::test]
async fn online_is_announced_once_per_generation_when_its_hello_applies() {
    let (client, mut rx, mut events) = scripted().await;
    client
        .set_desired_hello(Some(wsl_hello()))
        .expect("desired");
    let first = client.simulate_connected();
    assert_no_event(&mut events).await;

    applied(next_forward(&mut rx).await, first);
    assert_online_status(recv_payload(&mut events).await, first);
    client.await_hello(PROMPT).await.expect("applied");
    assert_eq!(
        client.applied_watched_repo_ids(),
        vec!["wsl_repo".to_string()]
    );

    client
        .set_desired_hello(Some(wsl_hello()))
        .expect("same payload");
    client.await_hello(PROMPT).await.expect("still applied");
    assert_no_forward(&mut rx).await;
    assert_no_event(&mut events).await;

    let second = client.simulate_connected();
    assert!(
        client.applied_watched_repo_ids().is_empty(),
        "applied ids are generation-scoped"
    );
    applied(next_forward(&mut rx).await, second);
    assert_online_status(recv_payload(&mut events).await, second);
    assert_no_event(&mut events).await;
}

#[tokio::test]
async fn connect_without_a_desired_hello_is_online_at_once() {
    let (client, mut rx, mut events) = scripted().await;
    let generation = client.simulate_connected();
    assert_online_status(recv_payload(&mut events).await, generation);
    assert_no_forward(&mut rx).await;
}

#[tokio::test]
async fn failed_hello_reaches_waiters_and_the_next_waiter_retries_once() {
    let (client, mut rx, _events) = scripted().await;
    let generation = client.simulate_connected();
    client
        .set_desired_hello(Some(wsl_hello()))
        .expect("desired");
    let request = next_forward(&mut rx).await;

    let waiters: Vec<_> = (0..2)
        .map(|_| {
            let client = client.clone();
            tokio::spawn(async move { client.await_hello(PROMPT).await })
        })
        .collect();
    tokio::task::yield_now().await;
    let _ = request
        .response_tx
        .send(Err(AgentError::new(WSL_BACKEND_TIMEOUT, "slow")));
    for waiter in waiters {
        let err = waiter.await.expect("waiter").expect_err("attempt failed");
        assert_eq!(err.code(), WSL_BACKEND_TIMEOUT);
    }

    let retry = tokio::spawn({
        let client = client.clone();
        async move { client.await_hello(PROMPT).await }
    });
    applied(next_forward(&mut rx).await, generation);
    retry.await.expect("retry").expect("retried hello applied");
    assert_no_forward(&mut rx).await;
}

#[test]
fn backend_version_must_support_root_identity() {
    for version in ["0.1.24", "0.1.25", "0.2.0", "1.0.0", "10.1.0"] {
        require_root_identity_version(version).expect("supported release");
    }
    for version in [
        "0.1.23",
        "0.0.99",
        "",
        "unknown",
        "0.1",
        "0.1.24.1",
        "0.1.+24",
        "0.1.24-dev",
    ] {
        let error = require_root_identity_version(version).expect_err("unsupported build");
        assert_eq!(error.code(), WSL_BACKEND_VERSION_MISMATCH);
        assert!(error.message().contains("0.1.24"));
    }
}
