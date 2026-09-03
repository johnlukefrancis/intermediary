// Path: crates/im_host_agent/src/server/shutdown_dispatch/tests.rs
// Description: Unit tests for the WSL-unavailable/outstanding-mutation shutdown decision

use std::net::TcpListener;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use im_agent::logging::{LogConfig, LogLevel, Logger};
use im_agent::protocol::{SourceControlActionCommand, SourceControlActionPayload, UiCommand};
use im_agent::server::EventBus;

use super::*;
use crate::error_codes::WSL_BACKEND_TIMEOUT;
use crate::wsl::{RequestLoopMessage, WslBackendClient};

async fn test_logger() -> Logger {
    Logger::init(LogConfig {
        log_dir: unique_log_dir(),
        min_level: LogLevel::Error,
        emit_stdio: false,
    })
    .await
    .expect("logger init")
}

fn unique_log_dir() -> PathBuf {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock");
    std::env::temp_dir().join(format!("im_host_agent_shutdown_dispatch_{}", now.as_nanos()))
}

/// A port nothing is listening on: bound and released immediately, so a
/// connection attempt refuses fast instead of hanging.
fn refused_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    listener.local_addr().expect("local addr").port()
}

fn client_with_no_backend(logger: Logger) -> WslBackendClient {
    let event_bus = EventBus::new(8);
    WslBackendClient::new(refused_port(), "token".to_string(), event_bus, logger)
}

/// No mutation was ever forwarded, so an unreachable backend is drained by
/// definition: the wait must not spend any of the deadline.
#[tokio::test]
async fn unavailable_backend_with_nothing_outstanding_drains_at_once() {
    let logger = test_logger().await;
    let client = client_with_no_backend(logger.clone());
    let deadline = Instant::now() + Duration::from_secs(5);

    let started = Instant::now();
    let outcome = drain_wsl_backend(Some(client), &logger, deadline).await;
    assert!(outcome.drained);
    assert_eq!(outcome.active_mutations, 0);
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "idle backend must not wait out the deadline"
    );
}

/// A mutation forwarded earlier that never got a confirmed answer leaves the
/// client believing it may still be running; an unreachable backend must not
/// be trusted as idle for it, and the wait must run out the deadline rather
/// than guess.
#[tokio::test]
async fn unavailable_backend_with_a_mutation_outstanding_waits_the_bound() {
    let logger = test_logger().await;
    let (client, mut request_rx) = WslBackendClient::with_request_channel();

    // The request loop's receiver, scripted: the mutation is received and
    // then held until its caller's timeout fires — sent with no answer back,
    // the one exit that leaves a mutation genuinely unaccounted for — while
    // every later request gets the answer an offline backend gives.
    let backend = tokio::spawn(async move {
        let mut held = Vec::new();
        while let Some(message) = request_rx.recv().await {
            match message {
                RequestLoopMessage::Forward(request)
                    if matches!(request.command, UiCommand::SourceControlAction(_)) =>
                {
                    held.push(request);
                }
                RequestLoopMessage::Forward(request) => {
                    let _ = request.response_tx.send(Err(AgentError::new(
                        WSL_BACKEND_UNAVAILABLE,
                        "WSL backend is not available",
                    )));
                }
                RequestLoopMessage::Cancel { .. } => {}
            }
        }
    });

    let command = UiCommand::SourceControlAction(SourceControlActionCommand {
        repo_id: "repo".to_string(),
        action: SourceControlActionPayload::Push,
    });
    let error = client
        .forward_command_with_timeout(command, Duration::from_millis(50))
        .await
        .expect_err("no answer ever came back");
    assert_eq!(error.code(), WSL_BACKEND_TIMEOUT);
    assert!(
        client.has_outstanding_mutations(),
        "a forward with no answer back proves nothing about the WSL side"
    );

    let bound = Duration::from_millis(200);
    let deadline = Instant::now() + bound;
    let started = Instant::now();
    let outcome = drain_wsl_backend(Some(client), &logger, deadline).await;

    assert!(!outcome.drained);
    assert_eq!(outcome.active_mutations, 1);
    assert!(
        started.elapsed() >= bound,
        "must spend the whole bound rather than give up early: {:?}",
        started.elapsed()
    );
    backend.await.expect("backend task");
}
