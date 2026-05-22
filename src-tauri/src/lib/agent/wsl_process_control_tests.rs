// Path: src-tauri/src/lib/agent/wsl_process_control_tests.rs
// Description: Tests for WSL process-control parsing helpers

use super::parse_pid_list;

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
