// Path: crates/im_host_agent/src/server/ws_server.rs
// Description: Host-agent WebSocket accept loop and connection dispatch

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use im_agent::error::AgentError;
use im_agent::logging::Logger;
use im_agent::server::EventBus;
use serde_json::json;
use tokio::net::TcpListener;
use tokio::sync::RwLock;

use crate::runtime::HostRuntime;

use super::connection::{handle_connection, ConnectionContext};
use super::handshake_auth::ConnectionHandshakeAuth;

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

    let shutdown = tokio::signal::ctrl_c();
    tokio::pin!(shutdown);

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
            signal_result = &mut shutdown => {
                if let Err(err) = signal_result {
                    config.logger.warn(
                        "Failed to listen for shutdown signal",
                        Some(json!({"error": err.to_string()})),
                    );
                }
                break;
            }
        }
    }

    config
        .logger
        .info("Host agent WebSocket server stopped", None);
    Ok(())
}
