// Path: crates/im_host_agent/src/server/connection.rs
// Description: Host-agent per-connection WebSocket handling and response serialization

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use futures_util::{SinkExt, StreamExt};
use im_agent::logging::Logger;
use im_agent::server::attach_runtime_identity_header;
use im_agent::server::EventBus;
use serde_json::json;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use tokio_tungstenite::tungstenite::Message;

use crate::runtime::HostRuntime;

use super::handshake_auth::{
    unauthorized_handshake_response, ConnectionHandshakeAuth, HandshakeRejectReason,
};
use super::request_dispatch::ConnectionRequests;

#[derive(Clone)]
pub struct ConnectionContext {
    pub runtime: Arc<RwLock<HostRuntime>>,
    pub logger: Logger,
    pub agent_version: String,
    pub event_bus: EventBus,
    pub handshake_auth: ConnectionHandshakeAuth,
    pub runtime_sha256: String,
}

pub async fn handle_connection(stream: TcpStream, peer: SocketAddr, ctx: ConnectionContext) {
    let handshake_reject_reason = Arc::new(Mutex::new(None::<HandshakeRejectReason>));
    let reject_reason_for_callback = Arc::clone(&handshake_reject_reason);
    let handshake_auth = ctx.handshake_auth.clone();
    let runtime_sha256 = ctx.runtime_sha256.clone();
    let ws_stream = match accept_hdr_async(stream, move |request: &Request, response: Response| {
        match handshake_auth.validate_request(request) {
            Ok(()) => Ok(attach_runtime_identity_header(response, &runtime_sha256)),
            Err(reason) => {
                if let Ok(mut slot) = reject_reason_for_callback.lock() {
                    *slot = Some(reason);
                }
                Err(unauthorized_handshake_response())
            }
        }
    })
    .await
    {
        Ok(stream) => stream,
        Err(err) => {
            let error_text = err.to_string();
            if is_expected_probe_handshake_error(&error_text) {
                ctx.logger.debug(
                    "Host-agent probe connection closed before websocket upgrade",
                    Some(json!({"peer": peer.to_string()})),
                );
            } else if let Some(reason) =
                handshake_reject_reason.lock().ok().and_then(|value| *value)
            {
                ctx.logger.warn(
                    "Host-agent WebSocket handshake rejected",
                    Some(json!({"peer": peer.to_string(), "reason": reason.as_log_reason()})),
                );
            } else {
                ctx.logger.warn(
                    "Host-agent WebSocket handshake failed",
                    Some(json!({"peer": peer.to_string()})),
                );
            }
            return;
        }
    };

    ctx.logger.info(
        "Host-agent client connected",
        Some(json!({"peer": peer.to_string()})),
    );

    let (mut sink, mut stream) = ws_stream.split();
    let (response_tx, mut response_rx) = mpsc::unbounded_channel::<Message>();
    // Slow clients lose bounded stream events with a seq gap; control events backpressure.
    let (mut event_rx, relay_task) = ctx.event_bus.relay(
        ctx.logger.clone(),
        peer.to_string(),
        "Host-agent broadcast lagged",
    );

    let writer_logger = ctx.logger.clone();
    let writer = tokio::spawn(async move {
        loop {
            let next = tokio::select! {
                biased;
                response = response_rx.recv() => response,
                event = event_rx.recv() => event.map(Message::Text),
            };

            let Some(message) = next else {
                break;
            };

            if let Err(err) = sink.send(message).await {
                writer_logger.warn(
                    "Failed to send host-agent WebSocket message",
                    Some(json!({"error": err.to_string()})),
                );
                break;
            }
        }
    });

    let mut requests = ConnectionRequests::default();
    while let Some(message) = stream.next().await {
        match message {
            Ok(Message::Text(text)) => {
                requests.receive(&text, &ctx, &response_tx);
            }
            Ok(Message::Binary(_)) => {
                ctx.logger.warn(
                    "Ignoring binary message",
                    Some(json!({"peer": peer.to_string()})),
                );
            }
            Ok(Message::Close(_)) => break,
            Ok(_) => {}
            Err(err) => {
                ctx.logger.warn(
                    "Host-agent WebSocket error",
                    Some(json!({"peer": peer.to_string(), "error": err.to_string()})),
                );
                break;
            }
        }
    }

    drop(requests);
    drop(response_tx);
    relay_task.abort();
    let _ = writer.await;

    ctx.logger.info(
        "Host-agent client disconnected",
        Some(json!({"peer": peer.to_string()})),
    );
}

fn is_expected_probe_handshake_error(error_text: &str) -> bool {
    error_text.contains("Handshake not finished")
}
