// Path: crates/im_host_agent/src/wsl/wsl_backend_client/tests.rs
// Description: Unit tests for WSL backend forwarding, cancellation, and outstanding-mutation tracking

use std::time::{SystemTime, UNIX_EPOCH};

use super::*;
use im_agent::logging::{LogConfig, LogLevel};
use im_agent::protocol::{ResponseEnvelope, SetOptionsResult};

/// A logger nobody reads: the decode site needs one.
pub(super) async fn quiet_logger() -> Logger {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock");
    Logger::init(LogConfig {
        log_dir: std::env::temp_dir()
            .join(format!("im_host_agent_wsl_client_{}", stamp.as_nanos())),
        min_level: LogLevel::Error,
        emit_stdio: false,
    })
    .await
    .expect("logger init")
}

pub(super) fn confirmed(request_id: String) -> ResponseEnvelope {
    ResponseEnvelope::ok(
        request_id,
        UiResponse::SetOptionsResult(SetOptionsResult {
            auto_stage_on_change: true,
        }),
    )
}

#[tokio::test]
async fn forward_command_timeout_enqueues_cancel_message() {
    let (client, mut request_rx) = WslBackendClient::with_request_channel();

    let recv_task = tokio::spawn(async move {
        let held_request = match request_rx.recv().await {
            Some(RequestLoopMessage::Forward(request)) => request,
            _ => panic!("expected forward request"),
        };
        let request_id = held_request.request_id.clone();

        let cancel = request_rx.recv().await.expect("cancel message");
        match cancel {
            RequestLoopMessage::Cancel {
                request_id: cancel_id,
            } => {
                assert_eq!(cancel_id, request_id);
            }
            _ => panic!("expected cancel message"),
        }

        drop(held_request);
    });

    let err = client
        .forward_command_with_timeout(UiCommand::Unknown, Duration::from_millis(10))
        .await
        .expect_err("timeout expected");
    assert_eq!(err.code(), WSL_BACKEND_TIMEOUT);
    assert!(
        err.message().contains("timed out"),
        "unexpected message: {}",
        err.message()
    );

    recv_task.await.expect("receiver task");
}

#[tokio::test]
async fn forwarded_response_preserves_serving_connection_generation() {
    let (client, mut request_rx) = WslBackendClient::with_request_channel();

    let response_task = tokio::spawn(async move {
        let request = match request_rx.recv().await {
            Some(RequestLoopMessage::Forward(request)) => request,
            _ => panic!("expected forward request"),
        };
        let response = ForwardedWslResponse {
            response: UiResponse::SetOptionsResult(SetOptionsResult {
                auto_stage_on_change: true,
            }),
            generation: 17,
        };
        request
            .response_tx
            .send(Ok(response))
            .expect("send response");
    });

    let forwarded = client
        .forward_command_with_timeout(UiCommand::Unknown, Duration::from_secs(1))
        .await
        .expect("forwarded response");
    assert_eq!(forwarded.generation, 17);
    assert!(matches!(
        forwarded.response,
        UiResponse::SetOptionsResult(SetOptionsResult {
            auto_stage_on_change: true
        })
    ));

    response_task.await.expect("response task");
}
