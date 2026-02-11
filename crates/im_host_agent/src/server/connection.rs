// Path: crates/im_host_agent/src/server/connection.rs
// Description: Host-agent per-connection WebSocket handling and response serialization

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use futures_util::{SinkExt, StreamExt};
use im_agent::error::to_response_error;
use im_agent::logging::Logger;
use im_agent::protocol::{EnvelopeKind, RequestEnvelope, ResponseEnvelope};
use im_agent::server::EventBus;
use serde_json::json;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use tokio_tungstenite::tungstenite::Message;

use crate::runtime::HostRuntime;

use super::dispatch;
use super::handshake_auth::{
    unauthorized_handshake_response, ConnectionHandshakeAuth, HandshakeRejectReason,
};

pub struct ConnectionContext {
    pub runtime: Arc<RwLock<HostRuntime>>,
    pub logger: Logger,
    pub agent_version: String,
    pub event_bus: EventBus,
    pub handshake_auth: ConnectionHandshakeAuth,
}

pub async fn handle_connection(stream: TcpStream, peer: SocketAddr, ctx: ConnectionContext) {
    let handshake_reject_reason = Arc::new(Mutex::new(None::<HandshakeRejectReason>));
    let reject_reason_for_callback = Arc::clone(&handshake_reject_reason);
    let handshake_auth = ctx.handshake_auth.clone();
    let ws_stream = match accept_hdr_async(stream, move |request: &Request, response: Response| {
        match handshake_auth.validate_request(request) {
            Ok(()) => Ok(response),
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
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<Message>();

    let writer_logger = ctx.logger.clone();
    let writer = tokio::spawn(async move {
        loop {
            let next = tokio::select! {
                biased;
                response = response_rx.recv() => response,
                event = event_rx.recv() => event,
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

    let mut broadcast_rx = ctx.event_bus.subscribe();
    let broadcast_logger = ctx.logger.clone();
    let event_tx_clone = event_tx.clone();
    let broadcast_task = tokio::spawn(async move {
        loop {
            match broadcast_rx.recv().await {
                Ok(text) => {
                    if event_tx_clone.send(Message::Text(text)).is_err() {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    broadcast_logger.warn(
                        "Host-agent broadcast lagged",
                        Some(json!({"skipped": skipped})),
                    );
                }
            }
        }
    });

    while let Some(message) = stream.next().await {
        match message {
            Ok(Message::Text(text)) => {
                if let Some(response) = handle_message(&text, &ctx).await {
                    let _ = response_tx.send(Message::Text(response));
                }
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

    drop(response_tx);
    drop(event_tx);
    broadcast_task.abort();
    let _ = writer.await;

    ctx.logger.info(
        "Host-agent client disconnected",
        Some(json!({"peer": peer.to_string()})),
    );
}

async fn handle_message(raw: &str, ctx: &ConnectionContext) -> Option<String> {
    let envelope: RequestEnvelope = match serde_json::from_str(raw) {
        Ok(envelope) => envelope,
        Err(err) => {
            ctx.logger.warn(
                "Invalid JSON message",
                Some(json!({"error": err.to_string()})),
            );
            return None;
        }
    };

    if envelope.kind != EnvelopeKind::Request {
        ctx.logger.warn(
            "Ignoring non-request envelope",
            Some(json!({"kind": format!("{:?}", envelope.kind)})),
        );
        return None;
    }

    let request_id = envelope.request_id.clone();
    let response = match dispatch::dispatch_command(envelope.payload, ctx).await {
        Ok(payload) => ResponseEnvelope::ok(request_id, payload),
        Err(err) => ResponseEnvelope::error(request_id, to_response_error(&err)),
    };

    match serde_json::to_string(&response) {
        Ok(text) => Some(text),
        Err(err) => {
            ctx.logger.error(
                "Failed to serialize response",
                Some(json!({"error": err.to_string()})),
            );
            None
        }
    }
}

fn is_expected_probe_handshake_error(error_text: &str) -> bool {
    error_text.contains("Handshake not finished")
}
