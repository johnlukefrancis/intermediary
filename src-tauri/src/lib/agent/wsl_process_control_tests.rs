// Path: src-tauri/src/lib/agent/wsl_process_control_tests.rs
// Description: Tests for WSL process-control parsing helpers

use super::parse_pid_list;

/// Manual end-to-end check of the real detection code path through a live `wsl.exe`. Ignored by
/// default (needs a running WSL and an im_agent-signed listener). See the runbook
/// `docs/commands/verify_wsl_port_detection.md` for the exact setup and invocation.
#[test]
#[ignore = "requires real wsl.exe + a live im_agent-signed listener on INTERMEDIARY_TEST_PORT"]
fn port_listener_detection_finds_live_agent_via_real_wsl() {
    let port: u16 = std::env::var("INTERMEDIARY_TEST_PORT")
        .expect("set INTERMEDIARY_TEST_PORT")
        .parse()
        .expect("INTERMEDIARY_TEST_PORT must be a u16");
    let pids = super::list_wsl_agent_pids_by_port_listener(None, port)
        .expect("real detection path should not error");
    eprintln!("port {port}: detected im_agent pids = {pids:?}");
    assert!(
        !pids.is_empty(),
        "real wsl.exe stdin path should detect the im_agent-signed listener on port {port}"
    );
}

#[test]
fn parse_pid_list_parses_unique_sorted_values() {
    let parsed = parse_pid_list("42\n7\n42\n\n9\n").expect("parsed");
    assert_eq!(parsed, vec![7, 9, 42]);
}

#[test]
fn parse_pid_list_rejects_non_numeric_entries() {
    let error = parse_pid_list("41\nabc\n").expect_err("expected parse failure");
    assert!(
        error.contains("Invalid pid entry"),
        "unexpected error: {error}"
    );
}
