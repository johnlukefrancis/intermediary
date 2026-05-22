// Path: crates/im_host_agent/src/wsl/wsl_backend_connection.rs
// Description: Connected WSL backend request loop and pending response handling

use std::collections::HashMap;

use futures_util::{SinkExt, StreamExt};
use im_agent::error::AgentError;
use im_agent::logging::Logger;
use im_agent::protocol::{EnvelopeKind, RequestEnvelope, UiResponse};
use im_agent::server::EventBus;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use super::wsl_backend_client::RequestLoopMessage;
use super::wsl_backend_messages::{
    fail_pending_requests, handle_backend_message, wsl_unavailable_error,
};

pub(super) async fn run_connected(
    stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    request_rx: &mut mpsc::UnboundedReceiver<RequestLoopMessage>,
    event_bus: &EventBus,
    logger: &Logger,
) {
    let (mut sink, mut read_stream) = stream.split();
    let mut pending: HashMap<String, oneshot::Sender<Result<UiResponse, AgentError>>> =
        HashMap::new();
    loop {
        tokio::select! {
            request = request_rx.recv() => {
                let Some(request) = request else {
                    fail_pending_requests(&mut pending, "WSL backend request loop closed");
                    break;
                };
                match request {
                    RequestLoopMessage::Forward(request) => {
                        let envelope = RequestEnvelope {
                            kind: EnvelopeKind::Request,
                            request_id: request.request_id.clone(),
                            payload: request.command,
                        };

                        let payload = match serde_json::to_string(&envelope) {
                            Ok(payload) => payload,
                            Err(err) => {
                                let _ = request.response_tx.send(Err(AgentError::internal(format!(
                                    "Failed to serialize WSL request: {err}"
                                ))));
                                continue;
                            }
                        };
                        pending.insert(request.request_id.clone(), request.response_tx);
                        if let Err(err) = sink.send(Message::Text(payload)).await {
                            fail_send_error(&mut pending, &request.request_id, err);
                            break;
                        }
                    }
                    RequestLoopMessage::Cancel { request_id } => {
                        pending.remove(&request_id);
                    }
                }
            }
            message = read_stream.next() => {
                let Some(message) = message else {
                    fail_pending_requests(&mut pending, "WSL backend disconnected");
                    break;
                };
                match message {
                    Ok(Message::Text(text)) => {
                        handle_backend_message(&text, &mut pending, event_bus, logger);
                    }
                    Ok(Message::Close(_)) => {
                        fail_pending_requests(&mut pending, "WSL backend closed connection");
                        break;
                    }
                    Ok(_) => {}
                    Err(err) => {
                        logger.warn(
                            "Error reading from WSL backend",
                            Some(serde_json::json!({"error": err.to_string()})),
                        );
                        fail_pending_requests(&mut pending, "WSL backend read failed");
                        break;
                    }
                }
            }
        }
    }
}

fn fail_send_error(
    pending: &mut HashMap<String, oneshot::Sender<Result<UiResponse, AgentError>>>,
    request_id: &str,
    err: tokio_tungstenite::tungstenite::Error,
) {
    if let Some(response_tx) = pending.remove(request_id) {
        let _ = response_tx.send(Err(wsl_unavailable_error(format!(
            "Failed to send request to WSL backend: {err}"
        ))));
    }
    fail_pending_requests(pending, "WSL backend disconnected while sending request");
}
