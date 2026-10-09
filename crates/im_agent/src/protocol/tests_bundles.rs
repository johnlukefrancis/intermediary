// Path: crates/im_agent/src/protocol/tests_bundles.rs
// Description: Bundle build and listing paths on command result and event wires

use serde_json::json;

use super::{AgentEvent, UiResponse};

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

    let response: UiResponse =
        serde_json::from_value(json).expect("parse legacy buildBundleResult");
    match response {
        UiResponse::BuildBundleResult(result) => {
            assert_eq!(result.host_path, "C:\\staging\\bundles\\repo.zip");
            assert_eq!(
                result.alias_host_path,
                "C:\\staging\\bundles\\repo_latest.zip"
            );
            assert_eq!(
                result.wsl_path.as_deref(),
                Some("/mnt/c/staging/bundles/repo.zip")
            );
        }
        _ => panic!("expected BuildBundleResult"),
    }
}

#[test]
fn build_bundle_result_accepts_host_and_legacy_paths_together() {
    let json = json!({
        "type": "buildBundleResult",
        "repoId": "repo",
        "presetId": "context",
        "hostPath": "C:\\staging\\bundles\\repo.zip",
        "windowsPath": "C:\\staging\\bundles\\repo.zip",
        "aliasHostPath": "C:\\staging\\bundles\\repo_latest.zip",
        "aliasWindowsPath": "C:\\staging\\bundles\\repo_latest.zip",
        "wslPath": "/mnt/c/staging/bundles/repo.zip",
        "bytes": 1024,
        "fileCount": 8,
        "builtAtIso": "2026-02-07T00:00:00Z"
    });

    let response: UiResponse =
        serde_json::from_value(json).expect("parse dual-path buildBundleResult");
    match response {
        UiResponse::BuildBundleResult(result) => {
            assert_eq!(result.host_path, "C:\\staging\\bundles\\repo.zip");
            assert_eq!(
                result.legacy_windows_path.as_deref(),
                Some("C:\\staging\\bundles\\repo.zip")
            );
            assert_eq!(
                result.alias_host_path,
                "C:\\staging\\bundles\\repo_latest.zip"
            );
            assert_eq!(
                result.legacy_alias_windows_path.as_deref(),
                Some("C:\\staging\\bundles\\repo_latest.zip")
            );
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
            assert_eq!(
                result.alias_host_path,
                "C:\\staging\\bundles\\repo_latest.zip"
            );
        }
        _ => panic!("expected BundleBuilt"),
    }
}

#[test]
fn bundle_built_event_accepts_host_and_legacy_paths_together() {
    let json = json!({
        "type": "bundleBuilt",
        "repoId": "repo",
        "presetId": "context",
        "hostPath": "C:\\staging\\bundles\\repo.zip",
        "windowsPath": "C:\\staging\\bundles\\repo.zip",
        "aliasHostPath": "C:\\staging\\bundles\\repo_latest.zip",
        "aliasWindowsPath": "C:\\staging\\bundles\\repo_latest.zip",
        "bytes": 1024,
        "fileCount": 8,
        "builtAtIso": "2026-02-07T00:00:00Z"
    });

    let event: AgentEvent = serde_json::from_value(json).expect("parse dual-path bundleBuilt");
    match event {
        AgentEvent::BundleBuilt(result) => {
            assert_eq!(result.host_path, "C:\\staging\\bundles\\repo.zip");
            assert_eq!(
                result.legacy_windows_path.as_deref(),
                Some("C:\\staging\\bundles\\repo.zip")
            );
            assert_eq!(
                result.alias_host_path,
                "C:\\staging\\bundles\\repo_latest.zip"
            );
            assert_eq!(
                result.legacy_alias_windows_path.as_deref(),
                Some("C:\\staging\\bundles\\repo_latest.zip")
            );
        }
        _ => panic!("expected BundleBuilt"),
    }
}

#[test]
fn bundle_built_event_rejects_conflicting_host_and_legacy_paths() {
    let json = json!({
        "type": "bundleBuilt",
        "repoId": "repo",
        "presetId": "context",
        "hostPath": "C:\\staging\\bundles\\new.zip",
        "windowsPath": "C:\\staging\\bundles\\old.zip",
        "aliasHostPath": "C:\\staging\\bundles\\repo_latest.zip",
        "aliasWindowsPath": "C:\\staging\\bundles\\repo_latest.zip",
        "bytes": 1024,
        "fileCount": 8,
        "builtAtIso": "2026-02-07T00:00:00Z"
    });

    let err =
        serde_json::from_value::<AgentEvent>(json).expect_err("conflicting paths should fail");
    assert!(
        err.to_string().contains("conflicting hostPath/windowsPath"),
        "unexpected error: {err}"
    );
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
            assert_eq!(
                result.bundles[0].host_path,
                "C:\\staging\\bundles\\repo.zip"
            );
        }
        _ => panic!("expected ListBundlesResult"),
    }
}

#[test]
fn list_bundles_result_accepts_host_and_legacy_paths_together() {
    let json = json!({
        "type": "listBundlesResult",
        "repoId": "repo",
        "presetId": "context",
        "bundles": [
            {
                "hostPath": "C:\\staging\\bundles\\repo.zip",
                "windowsPath": "C:\\staging\\bundles\\repo.zip",
                "fileName": "repo.zip",
                "bytes": 1024,
                "mtimeMs": 1700000000000_u64,
                "isLatestAlias": false
            }
        ]
    });

    let response: UiResponse =
        serde_json::from_value(json).expect("parse dual-path listBundlesResult");
    match response {
        UiResponse::ListBundlesResult(result) => {
            assert_eq!(
                result.bundles[0].host_path,
                "C:\\staging\\bundles\\repo.zip"
            );
            assert_eq!(
                result.bundles[0].legacy_windows_path.as_deref(),
                Some("C:\\staging\\bundles\\repo.zip")
            );
        }
        _ => panic!("expected ListBundlesResult"),
    }
}
