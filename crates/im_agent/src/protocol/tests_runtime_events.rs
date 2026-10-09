// Path: crates/im_agent/src/protocol/tests_runtime_events.rs
// Description: Backend status, repository topology and watcher error event wires

use serde_json::json;

use super::AgentEvent;

#[test]
fn wsl_backend_status_event_roundtrip() {
    let json = json!({
        "type": "wslBackendStatus",
        "status": "online",
        "generation": 4
    });

    let event: AgentEvent = serde_json::from_value(json).expect("parse wslBackendStatus");
    match event {
        AgentEvent::WslBackendStatus(result) => {
            assert_eq!(
                result.status,
                crate::protocol::WslBackendConnectionStatus::Online
            );
            assert_eq!(result.generation, 4);
        }
        _ => panic!("expected WslBackendStatus"),
    }
}

#[test]
fn repo_topology_changed_event_roundtrip() {
    let json = json!({
        "type": "repoTopologyChanged",
        "repoId": "trianglerain"
    });

    let event: AgentEvent = serde_json::from_value(json).expect("parse repoTopologyChanged");
    match event {
        AgentEvent::RepoTopologyChanged(result) => {
            assert_eq!(result.repo_id, "trianglerain");
        }
        _ => panic!("expected RepoTopologyChanged"),
    }
}

#[test]
fn watcher_mounted_windows_path_risk_error_code_roundtrip() {
    let json = json!({
        "type": "error",
        "scope": "watcher",
        "message": "Repo watcher is running on a mounted Windows path and may miss changes.",
        "details": {
            "code": "watcher_mounted_windows_path_risk",
            "docPath": "docs/usage/agent_wsl_bruised_states.md",
            "repoId": "screenshots",
            "rawCode": "MOUNTED_WINDOWS_PATH_RISK",
            "rawMessage": "Mounted Windows path: /mnt/c/Users/john/Pictures"
        }
    });

    let event: AgentEvent = serde_json::from_value(json).expect("parse watcher warning");
    match event {
        AgentEvent::Error(result) => {
            let details = result.details.expect("details");
            assert_eq!(
                details.code,
                Some(crate::protocol::AgentErrorCode::WatcherMountedWindowsPathRisk)
            );
            assert_eq!(details.repo_id.as_deref(), Some("screenshots"));
        }
        _ => panic!("expected Error"),
    }
}
