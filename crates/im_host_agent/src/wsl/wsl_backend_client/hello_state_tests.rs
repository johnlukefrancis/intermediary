// Path: crates/im_host_agent/src/wsl/wsl_backend_client/hello_state_tests.rs
// Description: Unit tests for WSL hello desired/applied/failed state and waiter decisions

use serde_json::json;

use super::{HelloWait, WslHelloState};
use im_agent::error::AgentError;
use im_agent::protocol::ClientHelloCommand;

fn hello(repo_id: &str, auto_stage: bool) -> ClientHelloCommand {
    ClientHelloCommand {
        config: json!({"repos": [{"repoId": repo_id, "root": {"kind": "wsl", "path": "/repo"}}]}),
        staging_host_root: "C:\\staging".to_string(),
        staging_wsl_root: Some("/mnt/c/staging".to_string()),
        auto_stage_on_change: Some(auto_stage),
    }
}

fn apply_current(state: &mut WslHelloState, generation: u64) -> bool {
    let attempt = state.begin_attempt(generation).expect("attempt");
    state.finish_applied(&attempt.fingerprint, generation, vec!["repo_a".to_string()])
}

#[test]
fn applied_payload_is_ready_only_for_its_generation() {
    let mut state = WslHelloState::default();
    assert!(state
        .set_desired(Some(hello("repo_a", false)))
        .expect("desired"));
    assert!(matches!(state.wait_status(1, 0), HelloWait::Kick));

    assert!(apply_current(&mut state, 1));
    assert!(matches!(state.wait_status(1, 0), HelloWait::Ready));
    assert!(state.begin_attempt(1).is_none());
    assert_eq!(
        state.applied_watched_repo_ids(1),
        vec!["repo_a".to_string()]
    );

    assert!(matches!(state.wait_status(2, 1), HelloWait::Kick));
    assert!(state.applied_watched_repo_ids(2).is_empty());
}

#[test]
fn unchanged_payload_does_not_report_a_change() {
    let mut state = WslHelloState::default();
    assert!(state
        .set_desired(Some(hello("repo_a", false)))
        .expect("first"));
    assert!(!state
        .set_desired(Some(hello("repo_a", false)))
        .expect("same"));
    assert!(state
        .set_desired(Some(hello("repo_a", true)))
        .expect("changed"));
    assert!(state.set_desired(None).expect("cleared"));
}

#[test]
fn reapplying_the_same_payload_on_a_generation_is_not_a_first_apply() {
    let mut state = WslHelloState::default();
    state
        .set_desired(Some(hello("repo_a", false)))
        .expect("desired");
    let first = state.begin_attempt(3).expect("attempt");
    assert!(state.finish_applied(&first.fingerprint, 3, Vec::new()));
    assert!(!state.finish_applied(&first.fingerprint, 3, Vec::new()));
}

#[test]
fn changed_payload_on_same_generation_is_pending_and_a_new_first_apply() {
    let mut state = WslHelloState::default();
    state
        .set_desired(Some(hello("repo_a", false)))
        .expect("desired");
    assert!(apply_current(&mut state, 1));
    state
        .set_desired(Some(hello("repo_a", true)))
        .expect("changed");
    assert!(matches!(state.wait_status(1, 1), HelloWait::Kick));
    assert!(apply_current(&mut state, 1));
}

#[test]
fn in_flight_attempt_keeps_waiters_pending() {
    let mut state = WslHelloState::default();
    state
        .set_desired(Some(hello("repo_a", false)))
        .expect("desired");
    let _attempt = state.begin_attempt(1).expect("attempt");
    assert!(state.begin_attempt(1).is_none(), "single in-flight attempt");
    assert!(matches!(state.wait_status(1, 0), HelloWait::Pending));
}

#[test]
fn failure_reaches_only_waiters_that_saw_it_start() {
    let mut state = WslHelloState::default();
    state
        .set_desired(Some(hello("repo_a", false)))
        .expect("desired");
    let attempt = state.begin_attempt(1).expect("attempt");
    let seen_before = state.finished_attempts();
    state.finish_failed(
        &attempt.fingerprint,
        &AgentError::new("WSL_BACKEND_TIMEOUT", "timed out"),
    );

    match state.wait_status(1, seen_before) {
        HelloWait::Failed(err) => assert_eq!(err.code(), "WSL_BACKEND_TIMEOUT"),
        other => panic!("expected failure, got {other:?}"),
    }
    let seen_after = state.finished_attempts();
    assert!(matches!(state.wait_status(1, seen_after), HelloWait::Kick));
}

#[test]
fn no_desired_hello_is_ready() {
    let state = WslHelloState::default();
    assert!(matches!(state.wait_status(0, 0), HelloWait::Ready));
}
