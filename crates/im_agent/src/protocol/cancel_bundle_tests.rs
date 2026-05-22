// Path: crates/im_agent/src/protocol/cancel_bundle_tests.rs
// Description: Protocol tests for cancellable bundle build messages

use serde_json::json;

use super::{UiCommand, UiResponse};

#[test]
fn cancel_bundle_build_command_roundtrips() {
    let json = json!({
        "type": "cancelBundleBuild",
        "repoId": "repo",
        "presetId": "context",
        "buildId": "build_1"
    });

    let command: UiCommand = serde_json::from_value(json).expect("parse cancelBundleBuild");
    match command {
        UiCommand::CancelBundleBuild(command) => {
            assert_eq!(command.repo_id, "repo");
            assert_eq!(command.preset_id, "context");
            assert_eq!(command.build_id, "build_1");
        }
        _ => panic!("expected CancelBundleBuild"),
    }
}

#[test]
fn cancel_bundle_build_result_roundtrips() {
    let json = json!({
        "type": "cancelBundleBuildResult",
        "repoId": "repo",
        "presetId": "context",
        "buildId": "build_1",
        "cancelled": true
    });

    let response: UiResponse = serde_json::from_value(json).expect("parse cancelBundleBuildResult");
    match response {
        UiResponse::CancelBundleBuildResult(result) => {
            assert_eq!(result.repo_id, "repo");
            assert_eq!(result.preset_id, "context");
            assert_eq!(result.build_id, "build_1");
            assert!(result.cancelled);
        }
        _ => panic!("expected CancelBundleBuildResult"),
    }
}
