// Path: crates/im_agent/src/logging/json_logger_tests.rs
// Description: Unit tests for the JSON logger's stdio mirroring switches

use super::{resolve_stdio_logging, resolve_stdio_target, StdioTarget};

#[test]
fn stdio_logging_defaults_to_enabled() {
    assert!(resolve_stdio_logging(None));
}

#[test]
fn stdio_logging_supports_disable_values() {
    assert!(!resolve_stdio_logging(Some("0".to_string())));
    assert!(!resolve_stdio_logging(Some("false".to_string())));
    assert!(!resolve_stdio_logging(Some("NO".to_string())));
    assert!(!resolve_stdio_logging(Some("off".to_string())));
}

#[test]
fn stdio_logging_supports_enable_values() {
    assert!(resolve_stdio_logging(Some("1".to_string())));
    assert!(resolve_stdio_logging(Some("true".to_string())));
    assert!(resolve_stdio_logging(Some("YES".to_string())));
    assert!(resolve_stdio_logging(Some("on".to_string())));
}

#[test]
fn stdio_logging_invalid_value_falls_back_to_enabled() {
    assert!(resolve_stdio_logging(Some("definitely".to_string())));
}

#[test]
fn disabled_stdio_suppresses_all_entry_output() {
    assert_eq!(resolve_stdio_target(false, "info"), StdioTarget::None);
    assert_eq!(resolve_stdio_target(false, "error"), StdioTarget::None);
}

#[test]
fn enabled_stdio_routes_errors_to_stderr() {
    assert_eq!(resolve_stdio_target(true, "error"), StdioTarget::Stderr);
    assert_eq!(resolve_stdio_target(true, "info"), StdioTarget::Stdout);
}
