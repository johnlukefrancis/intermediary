// Path: src-tauri/src/lib/agent/websocket_auth_tests.rs
// Description: Durable websocket authentication token persistence tests

use super::{
    create_tokens_if_absent, migrate_legacy_auth_file, read_or_create_tokens,
    AgentWebSocketAuthState, PersistedWsAuthTokens,
};
use std::fs;

#[test]
fn auth_state_can_be_built_before_an_app_handle_exists() {
    let dir = tempfile::tempdir().expect("tempdir");

    let state = AgentWebSocketAuthState::from_app_local_data(dir.path()).expect("auth state");

    assert!(!state.host_ws_token().is_empty());
    assert!(dir.path().join("ws_auth.json").is_file());
}

#[test]
fn read_or_create_tokens_preserves_existing_valid_auth_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("ws_auth.json");
    fs::write(
        &path,
        r#"{"hostWsToken":"existing-host","wslWsToken":"existing-wsl"}"#,
    )
    .expect("write auth");

    let tokens = read_or_create_tokens(&path).expect("tokens");

    assert_eq!(tokens.host_ws_token, "existing-host");
    assert_eq!(tokens.wsl_ws_token, "existing-wsl");
    assert_eq!(
        fs::read_to_string(&path).expect("read auth"),
        r#"{"hostWsToken":"existing-host","wslWsToken":"existing-wsl"}"#
    );
}

#[test]
fn read_or_create_tokens_replaces_invalid_existing_auth_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("ws_auth.json");
    fs::write(&path, r#"{"hostWsToken":"","wslWsToken":""}"#).expect("write auth");

    let tokens = read_or_create_tokens(&path).expect("tokens");

    assert!(!tokens.host_ws_token.is_empty());
    assert!(!tokens.wsl_ws_token.is_empty());
    let raw = fs::read_to_string(&path).expect("read auth");
    let persisted: PersistedWsAuthTokens = serde_json::from_str(&raw).expect("parse auth");
    assert_eq!(persisted.host_ws_token, tokens.host_ws_token);
    assert_eq!(persisted.wsl_ws_token, tokens.wsl_ws_token);
}

#[test]
fn migrate_legacy_auth_file_adopts_valid_legacy_tokens() {
    let dir = tempfile::tempdir().expect("tempdir");
    let legacy = dir.path().join("agent").join("ws_auth.json");
    fs::create_dir_all(legacy.parent().expect("legacy parent")).expect("mkdir legacy");
    fs::write(
        &legacy,
        r#"{"hostWsToken":"legacy-host","wslWsToken":"legacy-wsl"}"#,
    )
    .expect("write legacy");
    let new_path = dir.path().join("ws_auth.json");

    migrate_legacy_auth_file(&legacy, &new_path).expect("migrate");

    let tokens = read_or_create_tokens(&new_path).expect("tokens");
    assert_eq!(tokens.host_ws_token, "legacy-host");
    assert_eq!(tokens.wsl_ws_token, "legacy-wsl");
    assert!(!legacy.is_file(), "legacy file should be moved");
}

#[test]
fn migrate_legacy_auth_file_keeps_existing_new_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let legacy = dir.path().join("agent").join("ws_auth.json");
    fs::create_dir_all(legacy.parent().expect("legacy parent")).expect("mkdir legacy");
    fs::write(
        &legacy,
        r#"{"hostWsToken":"legacy-host","wslWsToken":"legacy-wsl"}"#,
    )
    .expect("write legacy");
    let new_path = dir.path().join("ws_auth.json");
    fs::write(
        &new_path,
        r#"{"hostWsToken":"current-host","wslWsToken":"current-wsl"}"#,
    )
    .expect("write current");

    migrate_legacy_auth_file(&legacy, &new_path).expect("migrate");

    let tokens = read_or_create_tokens(&new_path).expect("tokens");
    assert_eq!(tokens.host_ws_token, "current-host");
    assert!(legacy.is_file(), "legacy file must be left untouched");
}

#[test]
fn read_or_create_tokens_uses_concurrently_created_valid_auth_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("ws_auth.json");
    let concurrent = PersistedWsAuthTokens {
        host_ws_token: "concurrent-host".to_string(),
        wsl_ws_token: "concurrent-wsl".to_string(),
    };
    create_tokens_if_absent(&path, &concurrent).expect("create concurrent auth");

    let tokens = read_or_create_tokens(&path).expect("tokens");

    assert_eq!(tokens.host_ws_token, "concurrent-host");
    assert_eq!(tokens.wsl_ws_token, "concurrent-wsl");
    assert_eq!(
        fs::read_to_string(&path).expect("read auth"),
        r#"{"hostWsToken":"concurrent-host","wslWsToken":"concurrent-wsl"}"#
    );
}
