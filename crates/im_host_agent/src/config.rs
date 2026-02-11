// Path: crates/im_host_agent/src/config.rs
// Description: Host agent environment configuration parsing

use std::env;

use im_agent::logging::Logger;

const DEFAULT_HOST_PORT: u16 = 3141;
const HOST_WS_TOKEN_ENV: &str = "INTERMEDIARY_HOST_WS_TOKEN";
const WSL_WS_TOKEN_ENV: &str = "INTERMEDIARY_WSL_WS_TOKEN";
const HOST_WS_ALLOWED_ORIGINS_ENV: &str = "INTERMEDIARY_HOST_WS_ALLOWED_ORIGINS";

pub struct HostAgentConfig {
    pub host_port: Option<u16>,
    pub wsl_port: u16,
    pub agent_version: String,
    pub host_ws_token: String,
    pub wsl_ws_token: String,
    pub host_ws_allowed_origins: Vec<String>,
}

pub fn resolve_host_agent_config(logger: &Logger) -> Result<HostAgentConfig, String> {
    let host_port = resolve_port(
        env::var("INTERMEDIARY_AGENT_PORT").ok(),
        DEFAULT_HOST_PORT,
        "INTERMEDIARY_AGENT_PORT",
        logger,
    );

    let default_wsl_port = host_port.unwrap_or(DEFAULT_HOST_PORT).saturating_add(1);
    let wsl_port = resolve_port(
        env::var("INTERMEDIARY_WSL_AGENT_PORT").ok(),
        default_wsl_port,
        "INTERMEDIARY_WSL_AGENT_PORT",
        logger,
    )
    .unwrap_or(default_wsl_port);

    let agent_version = env::var("INTERMEDIARY_AGENT_VERSION")
        .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string());
    let host_ws_token = resolve_required_env(HOST_WS_TOKEN_ENV)?;
    let wsl_ws_token = resolve_required_env(WSL_WS_TOKEN_ENV)?;
    let host_ws_allowed_origins =
        resolve_allowed_origins(env::var(HOST_WS_ALLOWED_ORIGINS_ENV).ok());

    Ok(HostAgentConfig {
        host_port,
        wsl_port,
        agent_version,
        host_ws_token,
        wsl_ws_token,
        host_ws_allowed_origins,
    })
}

fn resolve_port(
    raw: Option<String>,
    default_port: u16,
    env_key: &str,
    logger: &Logger,
) -> Option<u16> {
    let raw = match raw {
        Some(raw) => raw,
        None => return Some(default_port),
    };

    let parsed: u16 = match raw.parse() {
        Ok(port) => port,
        Err(_) => {
            logger.warn(
                "Invalid port env value, using default",
                Some(serde_json::json!({"env": env_key, "raw": raw, "default": default_port})),
            );
            return Some(default_port);
        }
    };

    if parsed < 1024 {
        logger.warn(
            "Port env value out of range, using default",
            Some(serde_json::json!({"env": env_key, "raw": raw, "default": default_port})),
        );
        return Some(default_port);
    }

    Some(parsed)
}

fn resolve_required_env(env_key: &str) -> Result<String, String> {
    let value = env::var(env_key)
        .map_err(|_| format!("Missing required environment variable: {env_key}"))?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!(
            "Required environment variable {env_key} must not be empty"
        ));
    }
    Ok(trimmed.to_string())
}

fn resolve_allowed_origins(raw: Option<String>) -> Vec<String> {
    raw.map(|value| {
        value
            .split(',')
            .map(str::trim)
            .filter(|origin| !origin.is_empty())
            .map(str::to_string)
            .collect::<Vec<String>>()
    })
    .unwrap_or_default()
}
