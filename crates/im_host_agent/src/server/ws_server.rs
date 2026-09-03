// Path: crates/im_host_agent/src/server/ws_server.rs
// Description: Host-agent WebSocket accept loop and connection dispatch

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use im_agent::error::AgentError;
use im_agent::logging::Logger;
use im_agent::server::runtime_binary_sha256;
use im_agent::server::{finalize_shutdown, wait_for_shutdown_signal, EventBus};
use serde_json::json;
use tokio::net::TcpListener;
use tokio::sync::RwLock;

use crate::runtime::HostRuntime;

use super::connection::{handle_connection, ConnectionContext};
use super::handshake_auth::ConnectionHandshakeAuth;
use super::shutdown_dispatch::drain_for_shutdown;

const DEFAULT_PORT: u16 = 3141;

pub struct ServerConfig {
    pub port: Option<u16>,
    pub agent_version: String,
    pub host_ws_token: String,
    pub host_ws_allowed_origins: Vec<String>,
    pub runtime: Arc<RwLock<HostRuntime>>,
    pub logger: Logger,
}

pub async fn run_server(config: ServerConfig) -> Result<(), AgentError> {
    let runtime_sha256 =
        runtime_binary_sha256().map_err(|err| AgentError::new("RUNTIME_IDENTITY_FAILED", err))?;
    let port = config.port.unwrap_or(DEFAULT_PORT);
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|err| AgentError::new("BIND_FAILED", format!("Failed to bind: {err}")))?;

    let event_bus = EventBus::new(128);
    let handshake_auth =
        ConnectionHandshakeAuth::new(config.host_ws_token, config.host_ws_allowed_origins);

    config.logger.info(
        "Host agent WebSocket server started",
        Some(json!({"port": port})),
    );

    // SIGTERM and ctrl-c take the same route as the `shutdown` command: stop
    // accepting, drain the WSL backend and then this process, then return.
    let shutdown = wait_for_shutdown_signal(&config.logger);
    tokio::pin!(shutdown);
    let signal_reason;

    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((stream, peer)) => {
                        let ctx = ConnectionContext {
                            runtime: Arc::clone(&config.runtime),
                            logger: config.logger.clone(),
                            agent_version: config.agent_version.clone(),
                            event_bus: event_bus.clone(),
                            handshake_auth: handshake_auth.clone(),
                            runtime_sha256: runtime_sha256.clone(),
                        };
                        tokio::spawn(handle_connection(stream, peer, ctx));
                    }
                    Err(err) => {
                        config.logger.warn(
                            "Failed to accept host-agent connection",
                            Some(json!({"error": err.to_string()})),
                        );
                    }
                }
            }
            reason = &mut shutdown => {
                signal_reason = reason;
                break;
            }
        }
    }

    let targets = {
        let runtime = config.runtime.read().await;
        runtime.shutdown_targets()
    };
    let outcome = drain_for_shutdown(targets, signal_reason).await;
    finalize_shutdown(&config.logger, signal_reason, outcome).await;

    config.logger.info(
        "Host agent WebSocket server stopped",
        Some(json!({
            "signal": signal_reason,
            "drained": outcome.drained,
            "activeMutations": outcome.active_mutations,
        })),
    );
    Ok(())
}
