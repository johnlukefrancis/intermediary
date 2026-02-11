// Path: crates/im_host_agent/src/wsl/wsl_backend_client/tests.rs
// Description: Unit tests for WSL backend forwarded command timeout routing and pending-request cleanup

use super::*;
use im_agent::protocol::BundleSelection;

fn client_for_test(request_tx: mpsc::UnboundedSender<RequestLoopMessage>) -> WslBackendClient {
    WslBackendClient {
        request_tx,
        request_counter: Arc::new(AtomicU64::new(0)),
        connection_generation: Arc::new(AtomicU64::new(0)),
    }
}

#[test]
fn cancel_message_removes_pending_entry() {
    let (response_tx, _response_rx) = oneshot::channel::<Result<UiResponse, AgentError>>();
    let mut pending = HashMap::new();
    pending.insert("req_1".to_string(), response_tx);

    cancel_pending_request(&mut pending, "req_1");

    assert!(!pending.contains_key("req_1"));
}

#[tokio::test]
async fn forward_command_timeout_enqueues_cancel_message() {
    let (request_tx, mut request_rx) = mpsc::unbounded_channel();
    let client = client_for_test(request_tx);

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

#[test]
fn build_bundle_uses_extended_timeout_budget() {
    let command = UiCommand::BuildBundle(im_agent::protocol::BuildBundleCommand {
        repo_id: "repo".to_string(),
        preset_id: "context".to_string(),
        selection: BundleSelection {
            include_root: true,
            top_level_dirs: vec![],
            excluded_subdirs: vec![],
        },
        global_excludes: None,
    });

    assert_eq!(
        timeout_for_command(&command),
        FORWARD_REQUEST_TIMEOUT_BUILD_BUNDLE
    );
}

#[test]
fn stage_file_uses_default_timeout_budget() {
    let command = UiCommand::StageFile(im_agent::protocol::StageFileCommand {
        repo_id: "repo".to_string(),
        path: "src/main.rs".to_string(),
    });

    assert_eq!(
        timeout_for_command(&command),
        FORWARD_REQUEST_TIMEOUT_DEFAULT
    );
}
