// Path: crates/im_host_agent/src/wsl/wsl_backend_client.rs
// Description: Persistent WebSocket client for forwarding commands/events to the WSL backend agent

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use im_agent::error::AgentError;
use im_agent::logging::Logger;
use im_agent::protocol::{EnvelopeKind, RequestEnvelope, UiCommand, UiResponse};
use im_agent::server::EventBus;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

use super::wsl_backend_messages::{
    fail_pending_requests, handle_backend_message, wsl_unavailable_error,
};

const RECONNECT_DELAY_MS: u64 = 750;

#[derive(Clone)]
pub struct WslBackendClient {
    request_tx: mpsc::UnboundedSender<ForwardRequest>,
    request_counter: Arc<AtomicU64>,
}

struct ForwardRequest {
    request_id: String,
    command: UiCommand,
    response_tx: oneshot::Sender<Result<UiResponse, AgentError>>,
}

impl WslBackendClient {
    pub fn new(wsl_port: u16, event_bus: EventBus, logger: Logger) -> Self {
        let (request_tx, request_rx) = mpsc::unbounded_channel();
        let endpoint = format!("ws://127.0.0.1:{wsl_port}");

        tokio::spawn(run_client_loop(endpoint, request_rx, event_bus, logger));

        Self {
            request_tx,
            request_counter: Arc::new(AtomicU64::new(0)),
        }
    }

    pub async fn forward_command(&self, command: UiCommand) -> Result<UiResponse, AgentError> {
        let request_id = self.next_request_id();
        let (response_tx, response_rx) = oneshot::channel();

        self.request_tx
            .send(ForwardRequest {
                request_id,
                command,
                response_tx,
            })
            .map_err(|_| wsl_unavailable_error("WSL backend request loop is offline"))?;

        match response_rx.await {
            Ok(result) => result,
            Err(_) => Err(wsl_unavailable_error(
                "WSL backend closed before returning a response",
            )),
        }
    }

    fn next_request_id(&self) -> String {
        let next = self.request_counter.fetch_add(1, Ordering::Relaxed) + 1;
        format!("host_wsl_req_{next}")
    }
}

async fn run_client_loop(
    endpoint: String,
    mut request_rx: mpsc::UnboundedReceiver<ForwardRequest>,
    event_bus: EventBus,
    logger: Logger,
) {
    loop {
        match connect_async(endpoint.as_str()).await {
            Ok((stream, _)) => {
                logger.info(
                    "Connected to WSL backend",
                    Some(serde_json::json!({"endpoint": endpoint})),
                );
                run_connected(stream, &mut request_rx, &event_bus, &logger).await;
                logger.warn(
                    "Disconnected from WSL backend",
                    Some(serde_json::json!({"endpoint": endpoint})),
                );
            }
            Err(err) => {
                logger.warn(
                    "Failed to connect to WSL backend",
                    Some(serde_json::json!({"endpoint": endpoint, "error": err.to_string()})),
                );
            }
        }

        let retry_delay = sleep(Duration::from_millis(RECONNECT_DELAY_MS));
        tokio::pin!(retry_delay);

        loop {
            tokio::select! {
                _ = &mut retry_delay => break,
                request = request_rx.recv() => {
                    match request {
                        Some(request) => {
                            let _ = request.response_tx.send(Err(wsl_unavailable_error(
                                "WSL backend is not available",
                            )));
                        }
                        None => return,
                    }
                }
            }
        }
    }
}

async fn run_connected(
    stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    request_rx: &mut mpsc::UnboundedReceiver<ForwardRequest>,
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
                    if let Some(response_tx) = pending.remove(&request.request_id) {
                        let _ = response_tx.send(Err(wsl_unavailable_error(format!(
                            "Failed to send request to WSL backend: {err}"
                        ))));
                    }
                    fail_pending_requests(&mut pending, "WSL backend disconnected while sending request");
                    break;
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
