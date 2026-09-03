// Path: crates/im_host_agent/src/wsl/wsl_backend_client/tests.rs
// Description: Unit tests for WSL backend forwarding, cancellation, and outstanding-mutation tracking

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use super::client_loop::answer_offline;
use super::*;
use crate::error_codes::WSL_BACKEND_UNAVAILABLE;
use crate::wsl::wsl_backend_messages::handle_backend_message;
use im_agent::logging::{LogConfig, LogLevel};
use im_agent::protocol::{
    ResponseEnvelope, ResponseError, SetOptionsResult, SourceControlActionCommand,
    SourceControlActionPayload, SourceControlStatusCommand,
};

/// A logger nobody reads: the decode site needs one.
async fn quiet_logger() -> Logger {
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

fn confirmed(request_id: String) -> ResponseEnvelope {
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

fn action(payload: SourceControlActionPayload) -> UiCommand {
    UiCommand::SourceControlAction(SourceControlActionCommand {
        repo_id: "repo".to_string(),
        action: payload,
    })
}

/// Forwards one mutation and lets the decode site answer it with
/// `envelope_for(request_id)`, handing the client back so the test can read its
/// ledger. `after_timeout` models the envelope that crosses the timeout: the
/// responder waits for the Cancel the expiring wait enqueues — which is how it
/// learns the caller has already been answered — and only then decodes an
/// envelope that was on the wire before that Cancel could be acted on.
async fn mutation_answered_through_decode(
    timeout_duration: Duration,
    after_timeout: bool,
    envelope_for: fn(String) -> ResponseEnvelope,
) -> (WslBackendClient, Result<ForwardedWslResponse, AgentError>) {
    let logger = quiet_logger().await;
    let (client, mut request_rx) = WslBackendClient::with_request_channel();
    let outstanding = client.outstanding_mutations.clone();

    let respond_task = tokio::spawn(async move {
        let request = match request_rx.recv().await {
            Some(RequestLoopMessage::Forward(request)) => request,
            _ => panic!("expected forward request"),
        };
        let ForwardRequest {
            request_id,
            response_tx,
            ..
        } = *request;
        let mut pending = HashMap::from([(request_id.clone(), response_tx)]);
        if after_timeout {
            match request_rx.recv().await.expect("cancel message") {
                RequestLoopMessage::Cancel { request_id: id } => assert_eq!(id, request_id),
                _ => panic!("expected cancel message"),
            }
        }
        // Answered exactly as `run_connected` does: through the decode site.
        let text = serde_json::to_string(&envelope_for(request_id)).expect("serialize envelope");
        handle_backend_message(
            &text,
            &mut pending,
            &EventBus::new(4),
            &logger,
            1,
            &outstanding,
        );
    });

    let result = client
        .forward_command_with_timeout(action(SourceControlActionPayload::Push), timeout_duration)
        .await;
    respond_task.await.expect("respond task");
    (client, result)
}

fn rejected(request_id: String) -> ResponseEnvelope {
    ResponseEnvelope::error(
        request_id,
        ResponseError {
            code: "GIT_PUSH_REJECTED".to_string(),
            message: "remote rejected the push".to_string(),
            details: None,
        },
    )
}

/// A decoded result envelope is a confirmed answer: the mutation is done and
/// must stop being outstanding.
#[tokio::test]
async fn a_decoded_result_envelope_clears_the_outstanding_mutation() {
    let (client, result) =
        mutation_answered_through_decode(Duration::from_secs(5), false, confirmed).await;

    result.expect("forwarded response");
    assert!(!client.has_outstanding_mutations());
    assert_eq!(client.outstanding_mutation_count(), 0);
}

/// An error envelope is just as finished as a result one: the WSL agent
/// refused the push and is done, so a shutdown drain must not wait on it.
#[tokio::test]
async fn a_decoded_error_envelope_clears_the_outstanding_mutation() {
    let (client, result) =
        mutation_answered_through_decode(Duration::from_secs(5), false, rejected).await;

    let err = result.expect_err("the backend refused the push");
    assert_eq!(err.code(), "GIT_PUSH_REJECTED");
    assert!(!client.has_outstanding_mutations());
    assert_eq!(client.outstanding_mutation_count(), 0);
}

/// The race the timeout arm actually loses: the WSL agent's answer was already
/// on the wire when the host gave up, so it decodes after the caller has been
/// told `WSL_BACKEND_TIMEOUT` and after the Cancel was enqueued. Nothing is
/// left to deliver it to, yet it is still a confirmed answer — result or error
/// alike — so the ledger clears on the decode alone and the shutdown drain
/// does not wait on a mutation that is finished.
///
/// The other side of that race is `a_timed_out_mutation_stays_outstanding`
/// below: once the Cancel is acted on, no envelope for that request is ever
/// sent, because a `SourceControlAction` is cancelled passively and the WSL
/// agent suppresses the answer to a cancelled request rather than sending it
/// late.
#[tokio::test]
async fn an_envelope_that_crosses_the_timeout_clears_the_outstanding_mutation() {
    for envelope_for in [confirmed, rejected] {
        let (client, result) =
            mutation_answered_through_decode(Duration::from_millis(10), true, envelope_for).await;

        let err = result.expect_err("timeout expected");
        assert_eq!(err.code(), WSL_BACKEND_TIMEOUT);
        assert!(!client.has_outstanding_mutations());
    }
}

/// A mutation whose forward times out with no answer on the wire has proven
/// nothing: the WSL side may still be running it, and the Cancel the timeout
/// sends will make it suppress its eventual answer rather than send it, so
/// nothing will ever clear this id and it must stay outstanding.
#[tokio::test]
async fn a_timed_out_mutation_stays_outstanding() {
    let (client, mut request_rx) = WslBackendClient::with_request_channel();

    let recv_task = tokio::spawn(async move {
        let held_request = match request_rx.recv().await {
            Some(RequestLoopMessage::Forward(request)) => request,
            _ => panic!("expected forward request"),
        };
        let _ = request_rx.recv().await; // the cancel message
        drop(held_request);
    });

    let command = action(SourceControlActionPayload::Discard { targets: vec![] });
    let err = client
        .forward_command_with_timeout(command, Duration::from_millis(10))
        .await
        .expect_err("timeout expected");
    assert_eq!(err.code(), WSL_BACKEND_TIMEOUT);

    assert!(client.has_outstanding_mutations());
    assert_eq!(client.outstanding_mutation_count(), 1);
    recv_task.await.expect("receiver task");
}

/// Reads and other non-mutation commands are never tracked: only
/// `SourceControlAction` leaves residue Git cares about.
#[tokio::test]
async fn a_non_mutation_command_is_never_tracked_even_on_timeout() {
    let (client, mut request_rx) = WslBackendClient::with_request_channel();

    let recv_task = tokio::spawn(async move {
        let held_request = match request_rx.recv().await {
            Some(RequestLoopMessage::Forward(request)) => request,
            _ => panic!("expected forward request"),
        };
        let _ = request_rx.recv().await; // the cancel message
        drop(held_request);
    });

    let status = UiCommand::SourceControlStatus(SourceControlStatusCommand {
        repo_id: "repo".to_string(),
    });
    let _ = client
        .forward_command_with_timeout(status, Duration::from_millis(10))
        .await;

    assert!(!client.has_outstanding_mutations());
    recv_task.await.expect("receiver task");
}

/// The counterpart of the timeout above. An answer from the request loop's
/// offline arm proves the opposite of a timeout: the request never reached
/// the wire, so nothing of ours is running in WSL for it and a shutdown drain
/// must not hold its whole emergency bound open waiting for it.
#[tokio::test]
async fn an_offline_answer_clears_a_mutation_that_never_reached_the_wire() {
    let (client, mut request_rx) = WslBackendClient::with_request_channel();
    let outstanding = client.outstanding_mutations.clone();

    let loop_task = tokio::spawn(async move {
        match request_rx.recv().await {
            Some(RequestLoopMessage::Forward(request)) => answer_offline(*request, &outstanding),
            _ => panic!("expected forward request"),
        }
    });

    let command = action(SourceControlActionPayload::Discard { targets: vec![] });
    let err = client
        .forward_command_with_timeout(command, Duration::from_secs(5))
        .await
        .expect_err("the backend is offline");
    assert_eq!(err.code(), WSL_BACKEND_UNAVAILABLE);

    assert!(!client.has_outstanding_mutations());
    assert_eq!(client.outstanding_mutation_count(), 0);
    loop_task.await.expect("loop task");
}
