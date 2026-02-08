// Path: crates/im_agent/src/protocol/tests.rs
// Description: Protocol envelope serialization and backward-compat tests

use serde_json::json;

use super::{
    AgentEvent, ClientHelloCommand, ClientHelloResult, RequestEnvelope, ResponseEnvelope, UiCommand,
    UiResponse,
};
use crate::runtime::RepoRoot;

#[test]
fn request_response_roundtrip() {
    let command = UiCommand::ClientHello(ClientHelloCommand {
        config: json!({"autoStageGlobal": true}),
        staging_host_root: "C:\\staging".to_string(),
        staging_wsl_root: Some("/mnt/c/staging".to_string()),
        auto_stage_on_change: Some(false),
    });

    let request = RequestEnvelope {
        kind: super::EnvelopeKind::Request,
        request_id: "req_1".to_string(),
        payload: command,
    };

    let serialized_request = serde_json::to_string(&request).expect("serialize request");
    let parsed_request: RequestEnvelope =
        serde_json::from_str(&serialized_request).expect("parse request");

    assert_eq!(parsed_request.request_id, "req_1");

    let response_payload = UiResponse::ClientHelloResult(ClientHelloResult {
        agent_version: "0.1.0".to_string(),
        watched_repo_ids: vec![],
    });

    let response = ResponseEnvelope::ok("req_1", response_payload);
    let serialized_response = serde_json::to_string(&response).expect("serialize response");
    let parsed_response: ResponseEnvelope =
        serde_json::from_str(&serialized_response).expect("parse response");

    match parsed_response {
        ResponseEnvelope::Ok { request_id, .. } => assert_eq!(request_id, "req_1"),
        ResponseEnvelope::Error { .. } => panic!("expected ok response"),
    }
}

#[test]
fn legacy_client_hello_staging_win_root_alias() {
    let json = json!({
        "type": "clientHello",
        "config": {"autoStageGlobal": true},
        "stagingWinRoot": "C:\\old_staging",
        "stagingWslRoot": "/mnt/c/old_staging"
    });

    let command: UiCommand = serde_json::from_value(json).expect("parse legacy clientHello");
    match command {
        UiCommand::ClientHello(hello) => {
            assert_eq!(hello.staging_host_root, "C:\\old_staging");
            assert_eq!(hello.staging_wsl_root.as_deref(), Some("/mnt/c/old_staging"));
        }
        _ => panic!("expected ClientHello"),
    }
}

#[test]
fn client_hello_accepts_host_and_legacy_root_when_equal() {
    let json = json!({
        "type": "clientHello",
        "config": {"autoStageGlobal": true},
        "stagingHostRoot": "C:\\staging",
        "stagingWinRoot": "C:\\staging",
        "stagingWslRoot": "/mnt/c/staging"
    });

    let command: UiCommand = serde_json::from_value(json).expect("parse clientHello");
    match command {
        UiCommand::ClientHello(hello) => {
            assert_eq!(hello.staging_host_root, "C:\\staging");
            assert_eq!(hello.staging_wsl_root.as_deref(), Some("/mnt/c/staging"));
        }
        _ => panic!("expected ClientHello"),
    }
}

#[test]
fn client_hello_rejects_conflicting_host_and_legacy_root() {
    let json = json!({
        "type": "clientHello",
        "config": {"autoStageGlobal": true},
        "stagingHostRoot": "C:\\new",
        "stagingWinRoot": "C:\\old",
        "stagingWslRoot": "/mnt/c/staging"
    });

    let err = serde_json::from_value::<UiCommand>(json).expect_err("conflicting roots should fail");
    assert!(
        err.to_string()
            .contains("conflicting stagingHostRoot/stagingWinRoot values"),
        "unexpected error: {err}"
    );
}

#[test]
fn canonical_client_hello_serializes_host_root() {
    let hello = ClientHelloCommand {
        config: json!({}),
        staging_host_root: "C:\\staging".to_string(),
        staging_wsl_root: Some("/mnt/c/staging".to_string()),
        auto_stage_on_change: None,
    };

    let serialized = serde_json::to_value(&hello).expect("serialize");
    assert!(serialized.get("stagingHostRoot").is_some());
    assert!(serialized.get("stagingWinRoot").is_none());
}

#[test]
fn legacy_repo_root_kind_windows_alias() {
    let json = json!({"kind": "windows", "path": "C:\\repos\\myapp"});
    let root: RepoRoot = serde_json::from_value(json).expect("parse legacy windows root");
    assert_eq!(root.kind(), "host");
    assert_eq!(root.host_path(), Some("C:\\repos\\myapp"));
}

#[test]
fn canonical_repo_root_serializes_host_kind() {
    let root = RepoRoot::Host {
        path: "C:\\repos\\myapp".to_string(),
    };
    let serialized = serde_json::to_value(&root).expect("serialize");
    assert_eq!(serialized.get("kind").unwrap().as_str().unwrap(), "host");
}

#[test]
fn client_hello_without_wsl_root() {
    let json = json!({
        "type": "clientHello",
        "config": {"autoStageGlobal": true},
        "stagingHostRoot": "/Users/dev/staging"
    });

    let command: UiCommand = serde_json::from_value(json).expect("parse clientHello without wsl");
    match command {
        UiCommand::ClientHello(hello) => {
            assert_eq!(hello.staging_host_root, "/Users/dev/staging");
            assert!(hello.staging_wsl_root.is_none());
        }
        _ => panic!("expected ClientHello"),
    }
}

#[test]
fn legacy_stage_file_result_windows_path_alias() {
    let json = json!({
        "type": "stageFileResult",
        "repoId": "repo",
        "path": "src/main.ts",
        "windowsPath": "C:\\staging\\repo\\src\\main.ts",
        "wslPath": "/mnt/c/staging/repo/src/main.ts",
        "bytesCopied": 128,
        "mtimeMs": 1700000000000_u64
    });

    let response: UiResponse = serde_json::from_value(json).expect("parse legacy stageFileResult");
    match response {
        UiResponse::StageFileResult(result) => {
            assert_eq!(result.host_path, "C:\\staging\\repo\\src\\main.ts");
            assert_eq!(result.wsl_path.as_deref(), Some("/mnt/c/staging/repo/src/main.ts"));
        }
        _ => panic!("expected StageFileResult"),
    }
}

#[test]
fn legacy_build_bundle_result_windows_aliases() {
    let json = json!({
        "type": "buildBundleResult",
        "repoId": "repo",
        "presetId": "context",
        "windowsPath": "C:\\staging\\bundles\\repo.zip",
        "aliasWindowsPath": "C:\\staging\\bundles\\repo_latest.zip",
        "wslPath": "/mnt/c/staging/bundles/repo.zip",
        "bytes": 1024,
        "fileCount": 8,
        "builtAtIso": "2026-02-07T00:00:00Z"
    });

    let response: UiResponse = serde_json::from_value(json).expect("parse legacy buildBundleResult");
    match response {
        UiResponse::BuildBundleResult(result) => {
            assert_eq!(result.host_path, "C:\\staging\\bundles\\repo.zip");
            assert_eq!(result.alias_host_path, "C:\\staging\\bundles\\repo_latest.zip");
            assert_eq!(result.wsl_path.as_deref(), Some("/mnt/c/staging/bundles/repo.zip"));
        }
        _ => panic!("expected BuildBundleResult"),
    }
}

#[test]
fn legacy_bundle_built_event_windows_aliases() {
    let json = json!({
        "type": "bundleBuilt",
        "repoId": "repo",
        "presetId": "context",
        "windowsPath": "C:\\staging\\bundles\\repo.zip",
        "aliasWindowsPath": "C:\\staging\\bundles\\repo_latest.zip",
        "bytes": 1024,
        "fileCount": 8,
        "builtAtIso": "2026-02-07T00:00:00Z"
    });

    let event: AgentEvent = serde_json::from_value(json).expect("parse legacy bundleBuilt");
    match event {
        AgentEvent::BundleBuilt(result) => {
            assert_eq!(result.host_path, "C:\\staging\\bundles\\repo.zip");
            assert_eq!(result.alias_host_path, "C:\\staging\\bundles\\repo_latest.zip");
        }
        _ => panic!("expected BundleBuilt"),
    }
}

#[test]
fn legacy_list_bundles_result_windows_path_alias() {
    let json = json!({
        "type": "listBundlesResult",
        "repoId": "repo",
        "presetId": "context",
        "bundles": [
            {
                "windowsPath": "C:\\staging\\bundles\\repo.zip",
                "fileName": "repo.zip",
                "bytes": 1024,
                "mtimeMs": 1700000000000_u64,
                "isLatestAlias": false
            }
        ]
    });

    let response: UiResponse =
        serde_json::from_value(json).expect("parse legacy listBundlesResult");
    match response {
        UiResponse::ListBundlesResult(result) => {
            assert_eq!(result.bundles[0].host_path, "C:\\staging\\bundles\\repo.zip");
        }
        _ => panic!("expected ListBundlesResult"),
    }
}

#[test]
fn legacy_file_changed_staged_windows_path_alias() {
    let json = json!({
        "type": "fileChanged",
        "repoId": "repo",
        "path": "src/main.ts",
        "kind": "code",
        "changeType": "change",
        "mtime": "2026-02-07T00:00:00Z",
        "staged": {
            "windowsPath": "C:\\staging\\repo\\src\\main.ts",
            "wslPath": "/mnt/c/staging/repo/src/main.ts",
            "bytesCopied": 128,
            "mtimeMs": 1700000000000_u64
        }
    });

    let event: AgentEvent = serde_json::from_value(json).expect("parse legacy fileChanged");
    match event {
        AgentEvent::FileChanged(result) => {
            let staged = result.staged.expect("staged payload");
            assert_eq!(staged.host_path, "C:\\staging\\repo\\src\\main.ts");
            assert_eq!(staged.wsl_path.as_deref(), Some("/mnt/c/staging/repo/src/main.ts"));
        }
        _ => panic!("expected FileChanged"),
    }
}
