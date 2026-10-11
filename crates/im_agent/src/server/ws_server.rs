// Path: crates/im_agent/src/server/ws_server.rs
// Description: WebSocket accept loop and connection dispatch

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use serde_json::json;
use std::future::Future;
use std::pin::Pin;

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Notify, RwLock};

use crate::error::AgentError;
use crate::logging::Logger;
use crate::runtime::AgentRuntime;

use super::connection::{handle_connection, ConnectionContext};
use super::event_bus::EventBus;
use super::handshake_auth::ConnectionHandshakeAuth;
use super::runtime_identity::runtime_binary_sha256;
use super::shutdown::{drain_source_control, finalize_shutdown, wait_for_shutdown_signal};

const DEFAULT_PORT: u16 = 3141;

/// Asks the accept loop to stop and drop its listener; the `shutdown` command's first act.
#[derive(Clone, Default)]
pub struct ListenerRelease {
    notify: Arc<Notify>,
}

impl ListenerRelease {
    pub fn new() -> Self {
        Self::default()
    }

    /// Stored as a permit when the loop is not waiting yet, so a release is never lost.
    pub fn request(&self) {
        self.notify.notify_one();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AcceptStop {
    Signal(&'static str),
    Released,
}

impl AcceptStop {
    fn label(self) -> &'static str {
        match self {
            Self::Signal(reason) => reason,
            Self::Released => "shutdownCommand",
        }
    }
}

pub struct ServerConfig {
    pub port: Option<u16>,
    pub agent_version: String,
    pub ws_auth_token: String,
    pub runtime: Arc<RwLock<AgentRuntime>>,
    pub logger: Logger,
}

pub async fn run_server(config: ServerConfig) -> Result<(), AgentError> {
    let runtime_sha256 =
        runtime_binary_sha256().map_err(|err| AgentError::new("RUNTIME_IDENTITY_FAILED", err))?;
    let port = config.port.unwrap_or(DEFAULT_PORT);
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|err| AgentError::new("BIND_FAILED", format!("Failed to bind: {err}")))?;

    let event_bus = EventBus::new(128);
    let handshake_auth = ConnectionHandshakeAuth::new(config.ws_auth_token);

    config
        .logger
        .info("WebSocket server started", Some(json!({"port": port})));

    // Signals and the `shutdown` command both release the port first, so a respawn can
    // bind while this process drains; a command-led drain still yields to a later signal.
    let listener_release = ListenerRelease::new();
    let shutdown = wait_for_shutdown_signal(&config.logger);
    tokio::pin!(shutdown);
    let stop = accept_until_stop(
        listener,
        shutdown.as_mut(),
        &listener_release,
        &config.logger,
        |stream, peer| {
            let ctx = ConnectionContext {
                runtime: Arc::clone(&config.runtime),
                logger: config.logger.clone(),
                agent_version: config.agent_version.clone(),
                event_bus: event_bus.clone(),
                handshake_auth: handshake_auth.clone(),
                runtime_sha256: runtime_sha256.clone(),
                listener_release: listener_release.clone(),
            };
            tokio::spawn(handle_connection(stream, peer, ctx));
        },
    )
    .await;
    config.logger.info(
        "WebSocket listener released for shutdown",
        Some(json!({"port": port, "cause": stop.label()})),
    );
    let signal_reason = match stop {
        AcceptStop::Signal(reason) => reason,
        AcceptStop::Released => shutdown.await,
    };

    let locks = {
        let state = config.runtime.read().await;
        state.source_control_locks.clone()
    };
    let outcome = drain_source_control(&locks, &config.logger, signal_reason).await;
    finalize_shutdown(&config.logger, signal_reason, outcome).await;

    config.logger.info(
        "WebSocket server stopped",
        Some(json!({
            "signal": signal_reason,
            "drained": outcome.drained,
            "activeMutations": outcome.active_mutations,
        })),
    );
    Ok(())
}

/// Accepts until a signal or a release, and consumes the listener so the port is free on return.
pub(super) async fn accept_until_stop<S, A>(
    listener: TcpListener,
    mut shutdown: Pin<&mut S>,
    release: &ListenerRelease,
    logger: &Logger,
    mut on_accept: A,
) -> AcceptStop
where
    S: Future<Output = &'static str>,
    A: FnMut(TcpStream, SocketAddr),
{
    loop {
        tokio::select! {
            accept_result = listener.accept() => match accept_result {
                Ok((stream, peer)) => on_accept(stream, peer),
                Err(err) => logger.warn(
                    "Failed to accept connection",
                    Some(json!({"error": err.to_string()})),
                ),
            },
            reason = shutdown.as_mut() => return AcceptStop::Signal(reason),
            _ = release.notify.notified() => return AcceptStop::Released,
        }
    }
}

#[cfg(test)]
#[path = "ws_server_tests.rs"]
mod tests;
