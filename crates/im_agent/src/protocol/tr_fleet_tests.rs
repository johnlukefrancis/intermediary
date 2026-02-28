// Path: crates/im_agent/src/protocol/tr_fleet_tests.rs
// Description: TR fleet protocol command/response serialization tests

use serde_json::json;

use super::{
    GetTrFleetStatusResult, TrFleetActionKind, TrFleetActionResult, TrFleetEndpointError,
    TrFleetEndpointErrorCode, TrFleetWatchBackend, UiCommand, UiResponse,
};

#[test]
fn parses_get_tr_fleet_status_command() {
    let raw = json!({
        "type": "getTrFleetStatus"
    });

    let command: UiCommand = serde_json::from_value(raw).expect("parse getTrFleetStatus");
    match command {
        UiCommand::GetTrFleetStatus(_) => {}
        _ => panic!("expected GetTrFleetStatus command"),
    }
}

#[test]
fn parses_restart_watch_action_with_backend() {
    let raw = json!({
        "type": "trFleetAction",
        "action": "restartWatch",
        "port": 5604,
        "backend": "poll"
    });

    let command: UiCommand = serde_json::from_value(raw).expect("parse trFleetAction");
    match command {
        UiCommand::TrFleetAction(payload) => match payload.payload {
            super::TrFleetActionPayload::RestartWatch { port, backend } => {
                assert_eq!(port, 5604);
                assert!(matches!(backend, TrFleetWatchBackend::Poll));
            }
            _ => panic!("expected RestartWatch payload"),
        },
        _ => panic!("expected TrFleetAction command"),
    }
}

#[test]
fn tr_fleet_action_result_roundtrip() {
    let result = UiResponse::TrFleetActionResult(TrFleetActionResult {
        action: TrFleetActionKind::Rebuild,
        port: 5602,
        ok: false,
        status_code: Some(500),
        response_body: Some(json!({"ok": false, "error": "boom"})),
        error: Some(TrFleetEndpointError {
            code: TrFleetEndpointErrorCode::HttpError,
            message: "HTTP 500".to_string(),
            status_code: Some(500),
        }),
    });

    let serialized = serde_json::to_string(&result).expect("serialize trFleetActionResult");
    let parsed: UiResponse = serde_json::from_str(&serialized).expect("parse trFleetActionResult");

    match parsed {
        UiResponse::TrFleetActionResult(payload) => {
            assert!(matches!(payload.action, TrFleetActionKind::Rebuild));
            assert_eq!(payload.port, 5602);
            assert!(!payload.ok);
            assert_eq!(payload.status_code, Some(500));
        }
        _ => panic!("expected TrFleetActionResult response"),
    }
}

#[test]
fn get_tr_fleet_status_result_roundtrip() {
    let response = UiResponse::GetTrFleetStatusResult(GetTrFleetStatusResult {
        targets: vec![super::TrFleetTargetStatus {
            port: 5605,
            base_url: "http://127.0.0.1:5605".to_string(),
            status: Some(json!({"ok": true, "build": {"state": "idle"}})),
            doctor: None,
            status_error: None,
            doctor_error: Some(TrFleetEndpointError {
                code: TrFleetEndpointErrorCode::Unreachable,
                message: "connection refused".to_string(),
                status_code: None,
            }),
            fetched_at_ms: 1700000000000,
        }],
    });

    let serialized = serde_json::to_string(&response).expect("serialize getTrFleetStatusResult");
    let parsed: UiResponse =
        serde_json::from_str(&serialized).expect("parse getTrFleetStatusResult");

    match parsed {
        UiResponse::GetTrFleetStatusResult(payload) => {
            assert_eq!(payload.targets.len(), 1);
            assert_eq!(payload.targets[0].port, 5605);
            assert!(payload.targets[0].status.is_some());
            assert!(payload.targets[0].doctor_error.is_some());
        }
        _ => panic!("expected GetTrFleetStatusResult response"),
    }
}
