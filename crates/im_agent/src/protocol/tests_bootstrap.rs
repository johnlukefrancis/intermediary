// Path: crates/im_agent/src/protocol/tests_bootstrap.rs
// Description: Client hello staging and repository root wire contracts

use serde_json::json;

use super::{ClientHelloCommand, UiCommand};
use crate::runtime::RepoRoot;

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
            assert_eq!(
                hello.staging_wsl_root.as_deref(),
                Some("/mnt/c/old_staging")
            );
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
