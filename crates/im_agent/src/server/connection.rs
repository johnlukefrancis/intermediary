// Path: crates/im_agent/src/server/connection.rs
// Description: Per-connection WebSocket handling and request routing

use std::net::SocketAddr;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

use crate::error::to_response_error;
use crate::logging::Logger;
use crate::protocol::{EnvelopeKind, RequestEnvelope, ResponseEnvelope};
use crate::runtime::AgentRuntime;
use crate::server::EventBus;

mod dispatch;

pub struct ConnectionContext {
    pub runtime: Arc<RwLock<AgentRuntime>>,
    pub logger: Logger,
    pub agent_version: String,
    pub event_bus: EventBus,
}

pub async fn handle_connection(stream: TcpStream, peer: SocketAddr, ctx: ConnectionContext) {
    let ws_stream = match accept_async(stream).await {
        Ok(stream) => stream,
        Err(err) => {
            let error_text = err.to_string();
            if is_expected_probe_handshake_error(&error_text) {
                ctx.logger.debug(
                    "Probe connection closed before websocket upgrade",
                    Some(json!({"peer": peer.to_string(), "error": error_text})),
                );
            } else {
                ctx.logger.warn(
                    "WebSocket handshake failed",
                    Some(json!({"peer": peer.to_string(), "error": error_text})),
                );
            }
            return;
        }
    };

    ctx.logger
        .info("Client connected", Some(json!({"peer": peer.to_string()})));

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
                    "Failed to send WebSocket message",
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
                    broadcast_logger.warn("Broadcast lagged", Some(json!({"skipped": skipped})));
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
                    "WebSocket error",
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
        "Client disconnected",
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
    ctx.logger.debug(
        "Received command",
        Some(json!({"type": envelope.payload.command_type(), "requestId": request_id.clone()})),
    );
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
