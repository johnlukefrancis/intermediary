// Path: crates/im_host_agent/src/wsl/wsl_backend_client.rs
// Description: Persistent WebSocket client for forwarding commands/events to the WSL backend agent
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::error_codes::WSL_BACKEND_TIMEOUT;
use im_agent::error::AgentError;
use im_agent::logging::Logger;
use im_agent::protocol::{
    AgentEvent, SourceControlActionKind, UiCommand, UiResponse, WslBackendConnectionStatus,
    WslBackendStatusEvent,
};
use im_agent::server::EventBus;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{sleep, timeout};
use tokio_tungstenite::connect_async;

use super::wsl_backend_connection::run_connected;
use super::wsl_backend_messages::wsl_unavailable_error;
const RECONNECT_DELAY_MS: u64 = 750;
const FORWARD_REQUEST_TIMEOUT_DEFAULT: Duration = Duration::from_secs(60);
const FORWARD_REQUEST_TIMEOUT_CLIENT_HELLO: Duration = Duration::from_secs(12);
const FORWARD_REQUEST_TIMEOUT_BUILD_BUNDLE: Duration = Duration::from_secs(5 * 60);
// Source-control ladder: each host->WSL budget bounds a whole request, which
// may run several agent-side Git commands (a commit is status + commit +
// rev-parse + status, each with its own 20-120 s bound), so every tier sits
// above that agent-side worst case and strictly below the UI budget above it
// (120 / 150 / 300 / 360 s).
const FORWARD_REQUEST_TIMEOUT_SOURCE_CONTROL_READ: Duration = Duration::from_secs(90);
const FORWARD_REQUEST_TIMEOUT_SOURCE_CONTROL_INDEX: Duration = Duration::from_secs(120);
const FORWARD_REQUEST_TIMEOUT_SOURCE_CONTROL_COMMIT: Duration = Duration::from_secs(240);
const FORWARD_REQUEST_TIMEOUT_SOURCE_CONTROL_REMOTE: Duration = Duration::from_secs(300);

#[derive(Clone)]
pub struct WslBackendClient {
    request_tx: mpsc::UnboundedSender<RequestLoopMessage>,
    request_counter: Arc<AtomicU64>,
    connection_generation: Arc<AtomicU64>,
}

#[derive(Debug)]
pub struct ForwardedWslResponse {
    pub response: UiResponse,
    pub generation: u64,
}

pub(super) enum RequestLoopMessage {
    Forward(Box<ForwardRequest>),
    Cancel { request_id: String },
}

pub(super) struct ForwardRequest {
    pub(super) request_id: String,
    pub(super) command: UiCommand,
    pub(super) response_tx: oneshot::Sender<Result<ForwardedWslResponse, AgentError>>,
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

    pub async fn forward_command_with_generation(
        &self,
        command: UiCommand,
    ) -> Result<ForwardedWslResponse, AgentError> {
        let timeout_duration = timeout_for_command(&command);
        self.forward_command_with_timeout(command, timeout_duration)
            .await
    }
    async fn forward_command_with_timeout(
        &self,
        command: UiCommand,
        timeout_duration: Duration,
    ) -> Result<ForwardedWslResponse, AgentError> {
        let request_id = self.next_request_id();
        let (response_tx, response_rx) = oneshot::channel();

        self.request_tx
            .send(RequestLoopMessage::Forward(Box::new(ForwardRequest {
                request_id: request_id.clone(),
                command,
                response_tx,
            })))
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
        UiCommand::SourceControlStatus(_) | UiCommand::SourceControlDiff(_) => {
            FORWARD_REQUEST_TIMEOUT_SOURCE_CONTROL_READ
        }
        UiCommand::SourceControlAction(command) => match command.action.kind() {
            SourceControlActionKind::Stage
            | SourceControlActionKind::Unstage
            | SourceControlActionKind::Discard => FORWARD_REQUEST_TIMEOUT_SOURCE_CONTROL_INDEX,
            SourceControlActionKind::Commit => FORWARD_REQUEST_TIMEOUT_SOURCE_CONTROL_COMMIT,
            SourceControlActionKind::Push | SourceControlActionKind::Pull => {
                FORWARD_REQUEST_TIMEOUT_SOURCE_CONTROL_REMOTE
            }
        },
        UiCommand::SetOptions(_)
        | UiCommand::WatchRepo(_)
        | UiCommand::Refresh(_)
        | UiCommand::StageFile(_)
        | UiCommand::ReadTextFile(_)
        | UiCommand::ReadImageFile(_)
        | UiCommand::CancelBundleBuild(_)
        | UiCommand::GetRepoTopLevel(_)
        | UiCommand::ListRepoDirectory(_)
        | UiCommand::ListBundles(_)
        | UiCommand::GetTrFleetStatus(_)
        | UiCommand::TrFleetAction(_)
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
                run_connected(stream, &mut request_rx, &event_bus, &logger, generation).await;
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
#[cfg(test)]
mod tests;
