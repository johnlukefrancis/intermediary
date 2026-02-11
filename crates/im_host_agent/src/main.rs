// Path: crates/im_host_agent/src/main.rs
// Description: Host agent daemon entry point

use std::env;
use std::process;
use std::sync::Arc;

use tokio::sync::RwLock;

use im_agent::logging::{resolve_log_dir, LogConfig, LogLevel, Logger};
use im_host_agent::config::resolve_host_agent_config;
use im_host_agent::runtime::HostRuntime;
use im_host_agent::server::{run_server, ServerConfig};

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|arg| arg == "--version") {
        println!("{}", resolve_agent_version());
        return;
    }

    let log_dir = resolve_log_dir(env::var("INTERMEDIARY_AGENT_LOG_DIR").ok());
    let log_level = resolve_log_level(env::var("INTERMEDIARY_AGENT_LOG_LEVEL").ok());

    let logger = match Logger::init(LogConfig {
        log_dir,
        min_level: log_level,
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

    let config = match resolve_host_agent_config(&logger) {
        Ok(config) => config,
        Err(err) => {
            logger.error(
                "Failed to resolve host-agent configuration",
                Some(serde_json::json!({"error": err})),
            );
            process::exit(1);
        }
    };
    let runtime = Arc::new(RwLock::new(HostRuntime::new(
        config.wsl_port,
        config.wsl_ws_token.clone(),
        logger.clone(),
    )));

    if let Err(err) = run_server(ServerConfig {
        port: config.host_port,
        agent_version: config.agent_version,
        host_ws_token: config.host_ws_token,
        host_ws_allowed_origins: config.host_ws_allowed_origins,
        runtime,
        logger: logger.clone(),
    })
    .await
    {
        logger.error(
            "Host agent server failed",
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
