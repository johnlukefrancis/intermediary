// Path: src-tauri/src/lib/agent/types.rs
// Description: Types for supervising host agent lifecycle with optional Windows WSL backend

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

const TAURI_ORIGIN: &str = "tauri://localhost";
const TAURI_HTTP_ORIGIN: &str = "http://tauri.localhost";
const TAURI_HTTPS_ORIGIN: &str = "https://tauri.localhost";
const DEV_LOCALHOST_ORIGIN: &str = "http://localhost:5173";
const DEV_LOOPBACK_ORIGIN: &str = "http://127.0.0.1:5173";
const WS_AUTH_STATE_FILE: &str = "ws_auth.json";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSupervisorWslConfig {
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub distro: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSupervisorConfig {
    pub port: u16,
    pub auto_start: bool,
    #[serde(default)]
    pub wsl: Option<AgentSupervisorWslConfig>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSupervisorStatus {
    Started,
    AlreadyRunning,
    Disabled,
    Backoff,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSupervisorWslStatus {
    pub required: bool,
    pub port: u16,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSupervisorResult {
    pub status: AgentSupervisorStatus,
    pub port: u16,
    pub supports_wsl: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wsl: Option<AgentSupervisorWslStatus>,
    pub agent_dir: String,
    pub log_dir: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AgentWebSocketAuth {
    pub host_ws_token: String,
    pub wsl_ws_token: String,
    pub host_allowed_origins: Vec<String>,
}

impl AgentWebSocketAuth {
    fn from_tokens(tokens: PersistedWsAuthTokens) -> Self {
        Self {
            host_ws_token: tokens.host_ws_token,
            wsl_ws_token: tokens.wsl_ws_token,
            host_allowed_origins: default_host_allowed_origins(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedWsAuthTokens {
    host_ws_token: String,
    wsl_ws_token: String,
}

#[derive(Debug)]
pub struct AgentWebSocketAuthState {
    auth: AgentWebSocketAuth,
}

impl AgentWebSocketAuthState {
    pub fn from_app(app: &AppHandle) -> Result<Self, String> {
        let app_local_data = app
            .path()
            .app_local_data_dir()
            .map_err(|_| "Failed to resolve app local data directory".to_string())?;
        let agent_dir = app_local_data.join("agent");
        fs::create_dir_all(&agent_dir)
            .map_err(|err| format!("Failed to create agent directory: {err}"))?;

        let auth_path = agent_dir.join(WS_AUTH_STATE_FILE);
        let persisted = read_or_create_tokens(&auth_path)?;

        Ok(Self {
            auth: AgentWebSocketAuth::from_tokens(persisted),
        })
    }

    pub fn snapshot(&self) -> AgentWebSocketAuth {
        self.auth.clone()
    }

    pub fn host_ws_token(&self) -> &str {
        &self.auth.host_ws_token
    }
}

fn generate_ws_token() -> String {
    Uuid::new_v4().simple().to_string()
}

fn default_host_allowed_origins() -> Vec<String> {
    let mut host_allowed_origins = vec![
        TAURI_ORIGIN.to_string(),
        TAURI_HTTP_ORIGIN.to_string(),
        TAURI_HTTPS_ORIGIN.to_string(),
    ];

    if cfg!(debug_assertions) {
        host_allowed_origins.push(DEV_LOCALHOST_ORIGIN.to_string());
        host_allowed_origins.push(DEV_LOOPBACK_ORIGIN.to_string());
    }

    host_allowed_origins
}

fn read_or_create_tokens(path: &Path) -> Result<PersistedWsAuthTokens, String> {
    if let Some(valid) = read_valid_tokens(path)? {
        return Ok(valid);
    }

    let created = PersistedWsAuthTokens {
        host_ws_token: generate_ws_token(),
        wsl_ws_token: generate_ws_token(),
    };

    match create_tokens_if_absent(path, &created)? {
        TokenFileCreateResult::Created => Ok(created),
        TokenFileCreateResult::AlreadyExists => read_valid_tokens(path).and_then(|tokens| {
            tokens.map_or_else(
                || {
                    write_tokens(path, &created)?;
                    Ok(created)
                },
                Ok,
            )
        }),
    }
}

enum TokenFileCreateResult {
    Created,
    AlreadyExists,
}

fn read_valid_tokens(path: &Path) -> Result<Option<PersistedWsAuthTokens>, String> {
    if !path.is_file() {
        return Ok(None);
    }

    let raw = fs::read_to_string(path)
        .map_err(|err| format!("Failed to read websocket auth token file: {err}"))?;
    let Ok(parsed) = serde_json::from_str::<PersistedWsAuthTokens>(&raw) else {
        return Ok(None);
    };
    Ok(validate_tokens(parsed))
}

fn create_tokens_if_absent(
    path: &Path,
    tokens: &PersistedWsAuthTokens,
) -> Result<TokenFileCreateResult, String> {
    let raw = serde_json::to_vec(tokens)
        .map_err(|err| format!("Failed to serialize websocket auth token file: {err}"))?;
    let temp = unique_temp_path(path);
    fs::write(&temp, raw)
        .map_err(|err| format!("Failed to write websocket auth token temp file: {err}"))?;

    let link_result = fs::hard_link(&temp, path);
    let _ = fs::remove_file(&temp);
    match link_result {
        Ok(()) => Ok(TokenFileCreateResult::Created),
        Err(err) if err.kind() == ErrorKind::AlreadyExists => {
            Ok(TokenFileCreateResult::AlreadyExists)
        }
        Err(err) => Err(format!(
            "Failed to install websocket auth token file: {err}"
        )),
    }
}

fn write_tokens(path: &Path, tokens: &PersistedWsAuthTokens) -> Result<(), String> {
    let raw = serde_json::to_vec(tokens)
        .map_err(|err| format!("Failed to serialize websocket auth token file: {err}"))?;
    let temp = unique_temp_path(path);
    fs::write(&temp, raw)
        .map_err(|err| format!("Failed to write websocket auth token temp file: {err}"))?;
    fs::rename(&temp, path)
        .map_err(|err| format!("Failed to install websocket auth token file: {err}"))?;
    Ok(())
}

fn unique_temp_path(path: &Path) -> PathBuf {
    path.with_extension(format!("tmp.{}", std::process::id()))
}

fn validate_tokens(tokens: PersistedWsAuthTokens) -> Option<PersistedWsAuthTokens> {
    let host = tokens.host_ws_token.trim();
    let wsl = tokens.wsl_ws_token.trim();
    if host.is_empty() || wsl.is_empty() {
        return None;
    }
    Some(PersistedWsAuthTokens {
        host_ws_token: host.to_string(),
        wsl_ws_token: wsl.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::{create_tokens_if_absent, read_or_create_tokens, PersistedWsAuthTokens};
    use std::fs;

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
}
