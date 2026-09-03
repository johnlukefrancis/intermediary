// Path: crates/im_host_agent/src/wsl/wsl_backend_client/client_loop.rs
// Description: The WSL backend connect/reconnect loop and the answers it gives while the backend is unreachable

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use im_agent::logging::Logger;
use im_agent::protocol::{AgentEvent, WslBackendConnectionStatus, WslBackendStatusEvent};
use im_agent::server::EventBus;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio_tungstenite::connect_async;

use super::super::wsl_backend_connection::run_connected;
use super::super::wsl_backend_messages::wsl_unavailable_error;
use super::{untrack_outstanding, ForwardRequest, OutstandingMutations, RequestLoopMessage};

const RECONNECT_DELAY_MS: u64 = 750;

/// Answers one request the loop received while the backend was unreachable.
/// The request never reached the wire, so nothing of ours is running in WSL
/// for it: it is untracked first, because a mutation left outstanding here
/// would hold a shutdown drain open for the full emergency bound over a
/// request this host never sent.
pub(super) fn answer_offline(request: ForwardRequest, outstanding: &OutstandingMutations) {
    untrack_outstanding(outstanding, &request.request_id);
    let _ = request
        .response_tx
        .send(Err(wsl_unavailable_error("WSL backend is not available")));
}

pub(super) async fn run_client_loop(
    endpoint_connect: String,
    endpoint_log: String,
    mut request_rx: mpsc::UnboundedReceiver<RequestLoopMessage>,
    event_bus: EventBus,
    logger: Logger,
    connection_generation: Arc<AtomicU64>,
    outstanding_mutations: OutstandingMutations,
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
                run_connected(
                    stream,
                    &mut request_rx,
                    &event_bus,
                    &logger,
                    generation,
                    &outstanding_mutations,
                )
                .await;
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
                            answer_offline(*request, &outstanding_mutations);
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
