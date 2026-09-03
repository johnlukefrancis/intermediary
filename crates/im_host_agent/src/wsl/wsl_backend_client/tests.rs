// Path: crates/im_host_agent/src/wsl/wsl_backend_client/tests.rs
// Description: Unit tests for WSL backend forwarding, cancellation, and outstanding-mutation tracking

use super::client_loop::answer_offline;
use super::*;
use crate::error_codes::WSL_BACKEND_UNAVAILABLE;
use im_agent::protocol::{
    SetOptionsResult, SourceControlActionCommand, SourceControlActionPayload,
    SourceControlStatusCommand,
};

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

/// A forwarded mutation that gets a real answer (success or app-level error,
/// it does not matter which) is proven done and must stop being outstanding.
#[tokio::test]
async fn a_confirmed_mutation_response_clears_the_outstanding_set() {
    let (client, mut request_rx) = WslBackendClient::with_request_channel();

    let respond_task = tokio::spawn(async move {
        let request = match request_rx.recv().await {
            Some(RequestLoopMessage::Forward(request)) => request,
            _ => panic!("expected forward request"),
        };
        let response = ForwardedWslResponse {
            response: UiResponse::SetOptionsResult(SetOptionsResult {
                auto_stage_on_change: true,
            }),
            generation: 1,
        };
        request
            .response_tx
            .send(Ok(response))
            .expect("send response");
    });

    let command = action(SourceControlActionPayload::Push);
    client
        .forward_command_with_timeout(command, Duration::from_secs(1))
        .await
        .expect("forwarded response");

    assert!(!client.has_outstanding_mutations());
    assert_eq!(client.outstanding_mutation_count(), 0);
    respond_task.await.expect("respond task");
}

/// A mutation whose forward times out has proven nothing: the WSL side may
/// still be running it, so it must stay outstanding.
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
