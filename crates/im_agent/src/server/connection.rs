// Path: crates/im_agent/src/server/connection.rs
// Description: Per-connection WebSocket handling and request routing

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use tokio_tungstenite::tungstenite::Message;

use crate::error::to_response_error;
use crate::logging::Logger;
use crate::protocol::{InboundRequestEnvelope, ResponseEnvelope, UiCommand};
use crate::runtime::AgentRuntime;
use crate::server::attach_runtime_identity_header;
use crate::server::EventBus;

mod dispatch;
mod repo_commands;
mod request_cancellation;
use crate::server::handshake_auth::{
    unauthorized_handshake_response, ConnectionHandshakeAuth, HandshakeRejectReason,
};
use request_cancellation::RequestCancellation;

#[derive(Clone)]
pub struct ConnectionContext {
    pub runtime: Arc<RwLock<AgentRuntime>>,
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
                    "Probe connection closed before websocket upgrade",
                    Some(json!({"peer": peer.to_string()})),
                );
            } else if let Some(reason) =
                handshake_reject_reason.lock().ok().and_then(|value| *value)
            {
                ctx.logger.warn(
                    "WebSocket handshake rejected",
                    Some(json!({"peer": peer.to_string(), "reason": reason.as_log_reason()})),
                );
            } else {
                ctx.logger.warn(
                    "WebSocket handshake failed",
                    Some(json!({"peer": peer.to_string()})),
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
    let (request_done_tx, mut request_done_rx) = mpsc::unbounded_channel::<String>();
    let mut active_requests: HashMap<String, RequestCancellation> = HashMap::new();

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

    loop {
        let message = tokio::select! {
            request_id = request_done_rx.recv() => {
                if let Some(request_id) = request_id {
                    active_requests.remove(&request_id);
                }
                continue;
            }
            message = stream.next() => message,
        };
        let Some(message) = message else {
            break;
        };
        match message {
            Ok(Message::Text(text)) => {
                let envelope: InboundRequestEnvelope = match serde_json::from_str(&text) {
                    Ok(envelope) => envelope,
                    Err(err) => {
                        ctx.logger.warn(
                            "Invalid JSON message",
                            Some(json!({"error": err.to_string()})),
                        );
                        continue;
                    }
                };
                match envelope {
                    InboundRequestEnvelope::Request {
                        request_id,
                        payload,
                    } => {
                        if active_requests.contains_key(&request_id) {
                            ctx.logger.warn(
                                "Ignoring duplicate active request id",
                                Some(json!({"requestId": request_id})),
                            );
                            continue;
                        }
                        let request_ctx = ctx.clone();
                        let request_response_tx = response_tx.clone();
                        let completed_tx = request_done_tx.clone();
                        let active_id = request_id.clone();
                        let completed_id = request_id.clone();
                        let cancellation = RequestCancellation::for_command(&payload);
                        let request_cancellation = cancellation.clone();
                        tokio::spawn(async move {
                            if let Some(response) = handle_request(
                                request_id,
                                *payload,
                                &request_ctx,
                                &request_cancellation,
                            )
                            .await
                            {
                                let _ = request_response_tx.send(Message::Text(response));
                            }
                            let _ = completed_tx.send(completed_id);
                        });
                        active_requests.insert(active_id, cancellation);
                    }
                    InboundRequestEnvelope::Cancel { request_id } => {
                        if cancel_active_request(&active_requests, &request_id) {
                            ctx.logger.debug(
                                "Requested active request cancellation",
                                Some(json!({"requestId": request_id})),
                            );
                        }
                    }
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

    for request in active_requests.into_values() {
        request.cancel();
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

async fn handle_request(
    request_id: String,
    payload: UiCommand,
    ctx: &ConnectionContext,
    cancellation: &RequestCancellation,
) -> Option<String> {
    ctx.logger.debug(
        "Received command",
        Some(json!({"type": payload.command_type(), "requestId": request_id.clone()})),
    );
    let response = match dispatch::dispatch_command(payload, ctx, cancellation).await {
        Ok(payload) => ResponseEnvelope::ok(request_id, payload),
        Err(err) => ResponseEnvelope::error(request_id, to_response_error(&err)),
    };

    if cancellation.is_cancelled() {
        return None;
    }

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

fn cancel_active_request(
    active_requests: &HashMap<String, RequestCancellation>,
    request_id: &str,
) -> bool {
    let Some(request) = active_requests.get(request_id) else {
        return false;
    };
    request.cancel();
    true
}

fn is_expected_probe_handshake_error(error_text: &str) -> bool {
    error_text.contains("Handshake not finished")
}

#[cfg(test)]
#[path = "connection_tests.rs"]
mod tests;
