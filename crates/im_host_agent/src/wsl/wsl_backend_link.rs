// Path: crates/im_host_agent/src/wsl/wsl_backend_link.rs
// Description: Bounds on the host->WSL socket - connect+handshake deadline, ping cadence, silence and send limits

use std::time::Duration;

use futures_util::{Sink, SinkExt};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::tungstenite::{Error as WsError, Message};
use tokio_tungstenite::{connect_async_with_config, MaybeTlsStream, WebSocketStream};

pub(super) type WslSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// How long the host waits on the WSL socket before it calls the peer dead.
#[derive(Debug, Clone, Copy)]
pub(super) struct LinkLimits {
    /// TCP connect plus the WebSocket upgrade.
    pub connect: Duration,
    pub ping_interval: Duration,
    /// No inbound frame (pong, event or response) for this long drops the connection.
    pub silence: Duration,
    /// One outbound frame; a peer whose receive window stays shut is as dead as a silent one.
    pub send: Duration,
}

/// Two missed pongs plus margin: a half-open peer moves to a new generation within 25 s.
pub(super) const WSL_LINK_LIMITS: LinkLimits = LinkLimits {
    connect: Duration::from_secs(5),
    ping_interval: Duration::from_secs(10),
    silence: Duration::from_secs(25),
    send: Duration::from_secs(25),
};

/// One message is one frame; the 16 MiB default would reject a ~32 MiB two-sided image diff,
/// so frames get the 64 MiB message bound while per-side bounds stay in the agent.
const WSL_BACKEND_MAX_FRAME_BYTES: usize = 64 * 1024 * 1024;
const WSL_BACKEND_MAX_MESSAGE_BYTES: usize = 64 * 1024 * 1024;

pub(super) fn wsl_backend_ws_config() -> WebSocketConfig {
    WebSocketConfig {
        max_message_size: Some(WSL_BACKEND_MAX_MESSAGE_BYTES),
        max_frame_size: Some(WSL_BACKEND_MAX_FRAME_BYTES),
        ..WebSocketConfig::default()
    }
}

/// A listener that accepts but never upgrades (a wedged agent) fails here instead of hanging.
pub(super) async fn connect_bounded(endpoint: &str, limit: Duration) -> Result<WslSocket, String> {
    match timeout(
        limit,
        connect_async_with_config(endpoint, Some(wsl_backend_ws_config()), false),
    )
    .await
    {
        Ok(Ok((stream, _))) => Ok(stream),
        Ok(Err(err)) => Err(err.to_string()),
        Err(_) => Err(format!(
            "connect and handshake exceeded {}ms",
            limit.as_millis()
        )),
    }
}

pub(super) async fn send_bounded<S>(
    sink: &mut S,
    message: Message,
    limit: Duration,
) -> Result<(), String>
where
    S: Sink<Message, Error = WsError> + Unpin,
{
    match timeout(limit, sink.send(message)).await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(err)) => Err(err.to_string()),
        Err(_) => Err(format!("send exceeded {}ms", limit.as_millis())),
    }
}

#[cfg(test)]
#[path = "wsl_backend_link_tests.rs"]
mod tests;
