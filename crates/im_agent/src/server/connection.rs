// Path: crates/im_agent/src/server/connection.rs
// Description: Per-connection WebSocket handling and request routing

use std::net::SocketAddr;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

use crate::error::{to_response_error, AgentError};
use crate::logging::Logger;
use crate::protocol::{
    EnvelopeKind, RequestEnvelope, ResponseEnvelope, UiCommand, UiResponse,
};
use crate::runtime::AgentRuntime;

pub struct ConnectionContext {
    pub runtime: Arc<RwLock<AgentRuntime>>,
    pub logger: Logger,
    pub agent_version: String,
    pub broadcaster: broadcast::Sender<String>,
}

pub async fn handle_connection(stream: TcpStream, peer: SocketAddr, ctx: ConnectionContext) {
    let ws_stream = match accept_async(stream).await {
        Ok(stream) => stream,
        Err(err) => {
            ctx.logger.warn(
                "WebSocket handshake failed",
                Some(json!({"peer": peer.to_string(), "error": err.to_string()})),
            );
            return;
        }
    };

    ctx.logger.info(
        "Client connected",
        Some(json!({"peer": peer.to_string()})),
    );

    let (mut sink, mut stream) = ws_stream.split();
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Message>();

    let writer_logger = ctx.logger.clone();
    let writer = tokio::spawn(async move {
        while let Some(message) = out_rx.recv().await {
            if let Err(err) = sink.send(message).await {
                writer_logger.warn(
                    "Failed to send WebSocket message",
                    Some(json!({"error": err.to_string()})),
                );
                break;
            }
        }
    });

    let mut broadcast_rx = ctx.broadcaster.subscribe();
    let broadcast_logger = ctx.logger.clone();
    let out_tx_clone = out_tx.clone();
    let broadcast_task = tokio::spawn(async move {
        loop {
            match broadcast_rx.recv().await {
                Ok(text) => {
                    if out_tx_clone.send(Message::Text(text)).is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Closed) => break,
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    broadcast_logger.warn(
                        "Broadcast lagged",
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
                    let _ = out_tx.send(Message::Text(response));
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

    drop(out_tx);
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
    let response = match dispatch_command(envelope.payload, ctx).await {
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

async fn dispatch_command(command: UiCommand, ctx: &ConnectionContext) -> Result<UiResponse, AgentError> {
    match command {
        UiCommand::ClientHello(command) => {
            let mut state = ctx.runtime.write().await;
            let result = state.apply_client_hello(command, &ctx.agent_version);
            Ok(UiResponse::ClientHelloResult(result))
        }
        UiCommand::SetOptions(command) => {
            let mut state = ctx.runtime.write().await;
            let result = state.apply_set_options(command);
            Ok(UiResponse::SetOptionsResult(result))
        }
        UiCommand::Unknown => Err(AgentError::new(
            "UNKNOWN_COMMAND",
            "Unsupported command",
        )),
    }
}
