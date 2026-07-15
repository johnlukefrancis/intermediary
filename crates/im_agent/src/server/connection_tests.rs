// Path: crates/im_agent/src/server/connection_tests.rs
// Description: Request task cancellation tests for agent WebSocket connections

use std::collections::HashMap;

use crate::protocol::{BuildBundleCommand, BundleSelection, UiCommand};

use super::{cancel_active_request, request_cancellation::RequestCancellation};

#[test]
fn cancellation_signals_only_the_matching_active_request() {
    let first = RequestCancellation::for_command(&UiCommand::Unknown);
    let second = RequestCancellation::for_command(&UiCommand::Unknown);
    let active = HashMap::from([
        ("req_1".to_string(), first.clone()),
        ("req_2".to_string(), second.clone()),
    ]);

    assert!(cancel_active_request(&active, "req_1"));
    assert!(!cancel_active_request(&active, "missing"));
    assert!(first.is_cancelled());
    assert!(!second.is_cancelled());
    assert!(active.contains_key("req_1"));
    assert!(active.contains_key("req_2"));
}

#[test]
fn build_request_cancellation_signals_the_dispatch_token() {
    let command = UiCommand::BuildBundle(BuildBundleCommand {
        repo_id: "repo".to_string(),
        preset_id: "preset".to_string(),
        build_id: "build".to_string(),
        selection: BundleSelection {
            include_root: true,
            top_level_dirs: vec![],
            included_subdirs: vec![],
            excluded_subdirs: vec![],
            excluded_files: vec![],
        },
        global_excludes: None,
    });
    let cancellation = RequestCancellation::for_command(&command);
    let build_token = cancellation.bundle_token().expect("bundle token");
    let active = HashMap::from([("req_build".to_string(), cancellation)]);

    assert!(cancel_active_request(&active, "req_build"));
    assert!(build_token.is_cancelled());
}
