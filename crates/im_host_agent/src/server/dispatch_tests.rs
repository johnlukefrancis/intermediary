// Path: crates/im_host_agent/src/server/dispatch_tests.rs
// Description: Dispatch tests with a scripted WSL backend: hello never waits on WSL, commands share one hello

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use im_agent::logging::{LogConfig, LogLevel, Logger};
use im_agent::protocol::{
    ClientHelloCommand, ClientHelloResult, ListRepoDirectoryCommand, RefreshCommand,
    SetOptionsResult, UiCommand, UiResponse,
};
use im_agent::server::EventBus;
use serde_json::json;
use tokio::sync::{mpsc, RwLock};
use tokio::time::timeout;

use super::dispatch_command;
use crate::runtime::HostRuntime;
use crate::server::connection::ConnectionContext;
use crate::server::handshake_auth::ConnectionHandshakeAuth;
use crate::wsl::{ForwardRequest, ForwardedWslResponse, RequestLoopMessage, WslBackendClient};

const PROMPT: Duration = Duration::from_secs(2);

fn scratch_dir(label: &str) -> PathBuf {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock");
    std::env::temp_dir().join(format!(
        "im_host_agent_dispatch_{label}_{}",
        stamp.as_nanos()
    ))
}

async fn quiet_logger() -> Logger {
    Logger::init(LogConfig {
        log_dir: scratch_dir("logs"),
        min_level: LogLevel::Error,
        emit_stdio: false,
    })
    .await
    .expect("logger init")
}

struct Harness {
    ctx: ConnectionContext,
    client: WslBackendClient,
    backend: mpsc::UnboundedReceiver<RequestLoopMessage>,
    staging: String,
}

async fn harness() -> Harness {
    let logger = quiet_logger().await;
    let event_bus = EventBus::new(64);
    let (client, backend) =
        WslBackendClient::with_request_channel(event_bus.clone(), logger.clone());
    let runtime = HostRuntime::with_wsl_client_for_test(client.clone(), logger.clone());
    let ctx = ConnectionContext {
        runtime: Arc::new(RwLock::new(runtime)),
        logger,
        agent_version: "0.1.24".to_string(),
        event_bus,
        handshake_auth: ConnectionHandshakeAuth::new("token".to_string(), Vec::new()),
        runtime_sha256: "test".to_string(),
    };
    let staging = scratch_dir("staging").to_string_lossy().into_owned();
    Harness {
        ctx,
        client,
        backend,
        staging,
    }
}

fn wsl_hello(staging: &str) -> UiCommand {
    UiCommand::ClientHello(ClientHelloCommand {
        config: json!({"repos": [{"repoId": "wsl_repo", "root": {"kind": "wsl", "path": "/repo"}}]}),
        staging_host_root: staging.to_string(),
        staging_wsl_root: Some("/mnt/c/staging".to_string()),
        auto_stage_on_change: Some(false),
    })
}

fn wsl_refresh() -> UiCommand {
    UiCommand::Refresh(RefreshCommand {
        repo_id: "wsl_repo".to_string(),
    })
}

async fn hello_ok(ctx: &ConnectionContext, staging: &str) -> ClientHelloResult {
    let response = timeout(PROMPT, dispatch_command(wsl_hello(staging), ctx, None))
        .await
        .expect("host hello must not wait for WSL")
        .expect("host hello");
    match response {
        UiResponse::ClientHelloResult(result) => result,
        _ => panic!("expected clientHelloResult"),
    }
}

async fn next_forward(
    backend: &mut mpsc::UnboundedReceiver<RequestLoopMessage>,
) -> Box<ForwardRequest> {
    loop {
        match timeout(PROMPT, backend.recv())
            .await
            .expect("forward")
            .expect("loop open")
        {
            RequestLoopMessage::Forward(request) => return request,
            RequestLoopMessage::Cancel { .. } => {}
        }
    }
}

async fn assert_no_forward(backend: &mut mpsc::UnboundedReceiver<RequestLoopMessage>) {
    let next = timeout(Duration::from_millis(50), backend.recv()).await;
    assert!(next.is_err(), "unexpected extra WSL request");
}

fn answer(request: Box<ForwardRequest>, response: UiResponse, generation: u64) {
    let _ = request.response_tx.send(Ok(ForwardedWslResponse {
        response,
        generation,
    }));
}

#[tokio::test]
async fn host_hello_replies_while_the_wsl_hello_is_unanswered() {
    let mut h = harness().await;
    h.client.simulate_connected();

    let result = hello_ok(&h.ctx, &h.staging).await;
    assert!(
        result.watched_repo_ids.is_empty(),
        "no WSL hello applied yet"
    );
    let hello = next_forward(&mut h.backend).await;
    assert!(matches!(hello.command, UiCommand::ClientHello(_)));

    // A second hello is a host command too: it must not queue behind the pending WSL hello.
    hello_ok(&h.ctx, &h.staging).await;
    assert_no_forward(&mut h.backend).await;
}

#[tokio::test]
async fn wsl_commands_share_one_in_flight_hello_and_host_commands_proceed() {
    let mut h = harness().await;
    let generation = h.client.simulate_connected();
    hello_ok(&h.ctx, &h.staging).await;
    let hello = next_forward(&mut h.backend).await;
    assert!(matches!(hello.command, UiCommand::ClientHello(_)));

    let first = tokio::spawn({
        let ctx = h.ctx.clone();
        async move { dispatch_command(wsl_refresh(), &ctx, None).await }
    });
    let second = tokio::spawn({
        let ctx = h.ctx.clone();
        async move { dispatch_command(wsl_refresh(), &ctx, None).await }
    });
    assert_no_forward(&mut h.backend).await;
    hello_ok(&h.ctx, &h.staging).await;

    let applied = ClientHelloResult {
        agent_version: "0.1.24".to_string(),
        watched_repo_ids: vec!["wsl_repo".to_string()],
    };
    answer(hello, UiResponse::ClientHelloResult(applied), generation);
    for _ in 0..2 {
        let request = next_forward(&mut h.backend).await;
        assert!(
            matches!(request.command, UiCommand::Refresh(_)),
            "commands never send a hello"
        );
        let options = SetOptionsResult {
            auto_stage_on_change: false,
        };
        answer(request, UiResponse::SetOptionsResult(options), generation);
    }
    first.await.expect("first task").expect("first forward");
    second.await.expect("second task").expect("second forward");

    let result = hello_ok(&h.ctx, &h.staging).await;
    assert_eq!(result.watched_repo_ids, vec!["wsl_repo".to_string()]);
    assert_no_forward(&mut h.backend).await;
}

#[tokio::test]
async fn host_repo_command_returns_while_a_wsl_command_waits_on_a_silent_backend() {
    let mut h = harness().await;
    h.client.simulate_connected();
    let host_root = scratch_dir("host_repo");
    std::fs::create_dir_all(host_root.join("src")).expect("host repo dir");
    let hello = UiCommand::ClientHello(ClientHelloCommand {
        config: json!({"repos": [
            {"repoId": "host_repo", "root": {"kind": "host", "path": host_root.to_string_lossy()}},
            {"repoId": "wsl_repo", "root": {"kind": "wsl", "path": "/repo"}}
        ]}),
        staging_host_root: h.staging.clone(),
        staging_wsl_root: Some("/mnt/c/staging".to_string()),
        auto_stage_on_change: Some(false),
    });
    timeout(PROMPT, dispatch_command(hello, &h.ctx, None))
        .await
        .expect("host hello must not wait for WSL")
        .expect("host hello");
    let _unanswered_hello = next_forward(&mut h.backend).await;

    let waiting = tokio::spawn({
        let ctx = h.ctx.clone();
        async move { dispatch_command(wsl_refresh(), &ctx, None).await }
    });
    let listing = UiCommand::ListRepoDirectory(ListRepoDirectoryCommand {
        repo_id: "host_repo".to_string(),
        path: String::new(),
    });
    let response = timeout(PROMPT, dispatch_command(listing, &h.ctx, None))
        .await
        .expect("host command must not queue behind the WSL hello")
        .expect("host listing");
    assert!(matches!(response, UiResponse::ListRepoDirectoryResult(_)));
    assert!(
        !waiting.is_finished(),
        "the WSL command is still waiting on its hello"
    );
    waiting.abort();
}
