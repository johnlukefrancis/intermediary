// Path: src-tauri/src/lib/agent/websocket_auth.rs
// Description: Pre-WebView websocket authentication state and durable token persistence

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const TAURI_ORIGIN: &str = "tauri://localhost";
const TAURI_HTTP_ORIGIN: &str = "http://tauri.localhost";
const TAURI_HTTPS_ORIGIN: &str = "https://tauri.localhost";
const DEV_LOCALHOST_ORIGIN: &str = "http://localhost:5173";
const DEV_LOOPBACK_ORIGIN: &str = "http://127.0.0.1:5173";
const WS_AUTH_STATE_FILE: &str = "ws_auth.json";

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
    pub fn from_app_local_data(app_local_data: &Path) -> Result<Self, String> {
        fs::create_dir_all(app_local_data)
            .map_err(|err| format!("Failed to create app local data directory: {err}"))?;

        let auth_path = app_local_data.join(WS_AUTH_STATE_FILE);
        let legacy_auth_path = app_local_data.join("agent").join(WS_AUTH_STATE_FILE);
        migrate_legacy_auth_file(&legacy_auth_path, &auth_path)?;
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

/// Adopts a valid token file written under the legacy installer-wiped `agent/` directory.
fn migrate_legacy_auth_file(legacy_path: &Path, new_path: &Path) -> Result<(), String> {
    if new_path.is_file() || !legacy_path.is_file() {
        return Ok(());
    }
    if read_valid_tokens(legacy_path)?.is_none() {
        return Ok(());
    }

    if fs::rename(legacy_path, new_path).is_ok() {
        return Ok(());
    }

    fs::copy(legacy_path, new_path)
        .map_err(|err| format!("Failed to migrate websocket auth token file: {err}"))?;
    let _ = fs::remove_file(legacy_path);
    Ok(())
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
#[path = "websocket_auth_tests.rs"]
mod tests;
