// Path: crates/im_host_agent/src/wsl/wsl_backend_client.rs
// Description: Persistent WebSocket client for forwarding commands/events to the WSL backend agent
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::error_codes::WSL_BACKEND_TIMEOUT;
use futures_util::{SinkExt, StreamExt};
use im_agent::error::AgentError;
use im_agent::logging::Logger;
use im_agent::protocol::{
    AgentEvent, EnvelopeKind, RequestEnvelope, UiCommand, UiResponse, WslBackendConnectionStatus,
    WslBackendStatusEvent,
};
use im_agent::server::EventBus;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{sleep, timeout};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

use super::wsl_backend_messages::{
    fail_pending_requests, handle_backend_message, wsl_unavailable_error,
};
const RECONNECT_DELAY_MS: u64 = 750;
const FORWARD_REQUEST_TIMEOUT_DEFAULT: Duration = Duration::from_secs(60);
const FORWARD_REQUEST_TIMEOUT_CLIENT_HELLO: Duration = Duration::from_secs(12);
const FORWARD_REQUEST_TIMEOUT_BUILD_BUNDLE: Duration = Duration::from_secs(5 * 60);

#[derive(Clone)]
pub struct WslBackendClient {
    request_tx: mpsc::UnboundedSender<RequestLoopMessage>,
    request_counter: Arc<AtomicU64>,
    connection_generation: Arc<AtomicU64>,
}

enum RequestLoopMessage {
    Forward(ForwardRequest),
    Cancel { request_id: String },
}

struct ForwardRequest {
    request_id: String,
    command: UiCommand,
    response_tx: oneshot::Sender<Result<UiResponse, AgentError>>,
}
impl WslBackendClient {
    pub fn new(wsl_port: u16, wsl_ws_token: String, event_bus: EventBus, logger: Logger) -> Self {
        let (request_tx, request_rx) = mpsc::unbounded_channel();
        let endpoint_log = format!("ws://127.0.0.1:{wsl_port}");
        let endpoint_connect = format!("{endpoint_log}/?token={wsl_ws_token}");
        let connection_generation = Arc::new(AtomicU64::new(0));

        tokio::spawn(run_client_loop(
            endpoint_connect,
            endpoint_log,
            request_rx,
            event_bus,
            logger,
            connection_generation.clone(),
        ));

        Self {
            request_tx,
            request_counter: Arc::new(AtomicU64::new(0)),
            connection_generation,
        }
    }
    pub fn connection_generation(&self) -> u64 {
        self.connection_generation.load(Ordering::SeqCst)
    }

    pub async fn forward_command(&self, command: UiCommand) -> Result<UiResponse, AgentError> {
        let timeout_duration = timeout_for_command(&command);
        self.forward_command_with_timeout(command, timeout_duration)
            .await
    }
    async fn forward_command_with_timeout(
        &self,
        command: UiCommand,
        timeout_duration: Duration,
    ) -> Result<UiResponse, AgentError> {
        let request_id = self.next_request_id();
        let (response_tx, response_rx) = oneshot::channel();

        self.request_tx
            .send(RequestLoopMessage::Forward(ForwardRequest {
                request_id: request_id.clone(),
                command,
                response_tx,
            }))
            .map_err(|_| wsl_unavailable_error("WSL backend request loop is offline"))?;

        let timeout_ms = timeout_duration.as_millis();
        match timeout(timeout_duration, response_rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(wsl_unavailable_error(
                "WSL backend closed before returning a response",
            )),
            Err(_) => {
                let _ = self
                    .request_tx
                    .send(RequestLoopMessage::Cancel { request_id });
                Err(AgentError::new(
                    WSL_BACKEND_TIMEOUT,
                    format!("WSL backend timed out after {timeout_ms}ms waiting for response"),
                ))
            }
        }
    }

    fn next_request_id(&self) -> String {
        let next = self.request_counter.fetch_add(1, Ordering::Relaxed) + 1;
        format!("host_wsl_req_{next}")
    }
}
fn timeout_for_command(command: &UiCommand) -> Duration {
    match command {
        UiCommand::ClientHello(_) => FORWARD_REQUEST_TIMEOUT_CLIENT_HELLO,
        UiCommand::BuildBundle(_) => FORWARD_REQUEST_TIMEOUT_BUILD_BUNDLE,
        UiCommand::SetOptions(_)
        | UiCommand::WatchRepo(_)
        | UiCommand::Refresh(_)
        | UiCommand::StageFile(_)
        | UiCommand::GetRepoTopLevel(_)
        | UiCommand::ListBundles(_)
        | UiCommand::Unknown => FORWARD_REQUEST_TIMEOUT_DEFAULT,
    }
}

async fn run_client_loop(
    endpoint_connect: String,
    endpoint_log: String,
    mut request_rx: mpsc::UnboundedReceiver<RequestLoopMessage>,
    event_bus: EventBus,
    logger: Logger,
    connection_generation: Arc<AtomicU64>,
) {
    let mut logged_offline_connect_failure = false;
    let mut offline_emitted_generation: Option<u64> = None;
    loop {
        match connect_async(endpoint_connect.as_str()).await {
            Ok((stream, _)) => {
                let generation = connection_generation.fetch_add(1, Ordering::SeqCst) + 1;
                logged_offline_connect_failure = false;
                logger.info(
                    "Connected to WSL backend",
                    Some(serde_json::json!({"endpoint": &endpoint_log, "generation": generation})),
                );
                emit_wsl_backend_status(&event_bus, WslBackendConnectionStatus::Online, generation);
                run_connected(stream, &mut request_rx, &event_bus, &logger).await;
                logger.warn(
                    "Disconnected from WSL backend",
                    Some(serde_json::json!({"endpoint": &endpoint_log, "generation": generation})),
                );
                if offline_emitted_generation != Some(generation) {
                    emit_wsl_backend_status(
                        &event_bus,
                        WslBackendConnectionStatus::Offline,
                        generation,
                    );
                    offline_emitted_generation = Some(generation);
                }
            }
            Err(err) => {
                if !logged_offline_connect_failure {
                    logger.warn(
                        "Failed to connect to WSL backend",
                        Some(serde_json::json!({"endpoint": &endpoint_log, "error": err.to_string()})),
                    );
                    logged_offline_connect_failure = true;
                }
                let generation = connection_generation.load(Ordering::SeqCst);
                if offline_emitted_generation != Some(generation) {
                    emit_wsl_backend_status(
                        &event_bus,
                        WslBackendConnectionStatus::Offline,
                        generation,
                    );
                    offline_emitted_generation = Some(generation);
                }
            }
        }
        let retry_delay = sleep(Duration::from_millis(RECONNECT_DELAY_MS));
        tokio::pin!(retry_delay);
        loop {
            tokio::select! {
                _ = &mut retry_delay => break,
                request = request_rx.recv() => {
                    match request {
                        Some(RequestLoopMessage::Forward(request)) => {
                            let _ = request.response_tx.send(Err(wsl_unavailable_error(
                                "WSL backend is not available",
                            )));
                        }
                        Some(RequestLoopMessage::Cancel { .. }) => {}
                        None => return,
                    }
                }
            }
        }
    }
}
fn emit_wsl_backend_status(
    event_bus: &EventBus,
    status: WslBackendConnectionStatus,
    generation: u64,
) {
    event_bus.broadcast_event(AgentEvent::WslBackendStatus(WslBackendStatusEvent {
        status,
        generation,
    }));
}
async fn run_connected(
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
                            if let Some(response_tx) = pending.remove(&request.request_id) {
                                let _ = response_tx.send(Err(wsl_unavailable_error(format!(
                                    "Failed to send request to WSL backend: {err}"
                                ))));
                            }
                            fail_pending_requests(&mut pending, "WSL backend disconnected while sending request");
                            break;
                        }
                    }
                    RequestLoopMessage::Cancel { request_id } => {
                        cancel_pending_request(&mut pending, &request_id);
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
fn cancel_pending_request(
    pending: &mut HashMap<String, oneshot::Sender<Result<UiResponse, AgentError>>>,
    request_id: &str,
) {
    pending.remove(request_id);
}

#[cfg(test)]
mod tests;
