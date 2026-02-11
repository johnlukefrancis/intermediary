// Path: crates/im_agent/src/main.rs
// Description: WSL agent daemon entry point

use std::env;
use std::process;
use std::sync::Arc;

use tokio::sync::RwLock;

use im_agent::logging::{resolve_log_dir, resolve_stdio_logging, LogConfig, LogLevel, Logger};
use im_agent::runtime::AgentRuntime;
use im_agent::server::{run_server, ServerConfig};

const DEFAULT_PORT: u16 = 3141;
const WSL_WS_TOKEN_ENV: &str = "INTERMEDIARY_WSL_WS_TOKEN";

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|arg| arg == "--version") {
        println!("{}", resolve_agent_version());
        return;
    }

    let log_dir = resolve_log_dir(env::var("INTERMEDIARY_AGENT_LOG_DIR").ok());
    let log_level = resolve_log_level(env::var("INTERMEDIARY_AGENT_LOG_LEVEL").ok());
    let emit_stdio = resolve_stdio_logging(env::var("INTERMEDIARY_AGENT_STDIO_LOGGING").ok());

    let logger = match Logger::init(LogConfig {
        log_dir,
        min_level: log_level,
        emit_stdio,
    })
    .await
    {
        Ok(logger) => logger,
        Err(err) => {
            eprintln!(
                "{{\"level\":\"error\",\"msg\":\"Failed to init logger\",\"error\":\"{}\"}}",
                err
            );
            process::exit(1);
        }
    };

    let port = resolve_port(env::var("INTERMEDIARY_AGENT_PORT").ok(), &logger);
    let ws_auth_token = match resolve_ws_auth_token() {
        Ok(token) => token,
        Err(err) => {
            logger.error(
                "Agent startup failed due to invalid websocket auth configuration",
                Some(serde_json::json!({"error": err})),
            );
            process::exit(1);
        }
    };
    let runtime = Arc::new(RwLock::new(AgentRuntime::new()));
    let agent_version = resolve_agent_version();

    if let Err(err) = run_server(ServerConfig {
        port,
        agent_version,
        ws_auth_token,
        runtime,
        logger: logger.clone(),
    })
    .await
    {
        logger.error(
            "Agent server failed",
            Some(serde_json::json!({"error": err.to_string()})),
        );
        process::exit(1);
    }
}

fn resolve_agent_version() -> String {
    env::var("INTERMEDIARY_AGENT_VERSION").unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string())
}

fn resolve_log_level(raw: Option<String>) -> LogLevel {
    raw.and_then(|value| value.parse().ok())
        .unwrap_or(LogLevel::Info)
}

fn resolve_port(raw: Option<String>, logger: &Logger) -> Option<u16> {
    let raw = match raw {
        Some(raw) => raw,
        None => return Some(DEFAULT_PORT),
    };

    let parsed: u16 = match raw.parse() {
        Ok(port) => port,
        Err(_) => {
            logger.warn(
                "Invalid INTERMEDIARY_AGENT_PORT, using default",
                Some(serde_json::json!({"raw": raw})),
            );
            return Some(DEFAULT_PORT);
        }
    };

    if parsed < 1024 {
        logger.warn(
            "INTERMEDIARY_AGENT_PORT out of range, using default",
            Some(serde_json::json!({"raw": raw})),
        );
        return Some(DEFAULT_PORT);
    }

    Some(parsed)
}

fn resolve_ws_auth_token() -> Result<String, String> {
    let value = env::var(WSL_WS_TOKEN_ENV)
        .map_err(|_| format!("Missing required environment variable: {WSL_WS_TOKEN_ENV}"))?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!(
            "Required environment variable {WSL_WS_TOKEN_ENV} must not be empty"
        ));
    }
    Ok(trimmed.to_string())
}
