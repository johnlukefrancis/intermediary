// Path: crates/im_host_agent/src/wsl/wsl_backend_connection.rs
// Description: Connected WSL backend request loop, ping liveness, and pending response handling

use std::collections::HashMap;

use futures_util::StreamExt;
use im_agent::error::AgentError;
use im_agent::logging::Logger;
use im_agent::protocol::InboundRequestEnvelope;
use im_agent::server::EventBus;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{interval_at, sleep, Instant, MissedTickBehavior};
use tokio_tungstenite::tungstenite::Message;

use super::wsl_backend_client::{
    untrack_outstanding, ForwardedWslResponse, OutstandingMutations, RequestLoopMessage,
};
use super::wsl_backend_link::{send_bounded, LinkLimits, WslSocket};
use super::wsl_backend_messages::{
    fail_pending_requests, handle_backend_message, wsl_unavailable_error,
};

#[allow(clippy::too_many_arguments)]
pub(super) async fn run_connected(
    stream: WslSocket,
    request_rx: &mut mpsc::UnboundedReceiver<RequestLoopMessage>,
    event_bus: &EventBus,
    logger: &Logger,
    generation: u64,
    outstanding_mutations: &OutstandingMutations,
    limits: LinkLimits,
) {
    let (mut sink, mut read_stream) = stream.split();
    let mut pending: HashMap<String, oneshot::Sender<Result<ForwardedWslResponse, AgentError>>> =
        HashMap::new();
    let mut ping = interval_at(Instant::now() + limits.ping_interval, limits.ping_interval);
    ping.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let silence = sleep(limits.silence);
    tokio::pin!(silence);
    loop {
        tokio::select! {
            _ = &mut silence => {
                logger.warn(
                    "WSL backend stopped answering pings; dropping the connection",
                    Some(serde_json::json!({
                        "generation": generation,
                        "silenceMs": limits.silence.as_millis(),
                    })),
                );
                fail_pending_requests(&mut pending, "WSL backend stopped answering pings");
                break;
            }
            _ = ping.tick() => {
                if let Err(err) = send_bounded(&mut sink, Message::Ping(Vec::new()), limits.send).await {
                    fail_pending_requests(&mut pending, format!("Failed to ping WSL backend: {err}"));
                    break;
                }
            }
            request = request_rx.recv() => {
                let Some(request) = request else {
                    fail_pending_requests(&mut pending, "WSL backend request loop closed");
                    break;
                };
                match request {
                    RequestLoopMessage::Forward(request) => {
                        let envelope = InboundRequestEnvelope::request(
                            request.request_id.clone(),
                            request.command,
                        );

                        let payload = match serde_json::to_string(&envelope) {
                            Ok(payload) => payload,
                            Err(err) => {
                                // Answered without ever being sent, so nothing
                                // of ours is running in WSL for it either.
                                untrack_outstanding(outstanding_mutations, &request.request_id);
                                let _ = request.response_tx.send(Err(AgentError::internal(format!(
                                    "Failed to serialize WSL request: {err}"
                                ))));
                                continue;
                            }
                        };
                        pending.insert(request.request_id.clone(), request.response_tx);
                        if let Err(err) =
                            send_bounded(&mut sink, Message::Text(payload), limits.send).await
                        {
                            fail_send_error(&mut pending, &request.request_id, err);
                            break;
                        }
                    }
                    RequestLoopMessage::Cancel { request_id } => {
                        pending.remove(&request_id);
                        let envelope = InboundRequestEnvelope::cancel(request_id);
                        let payload = match serde_json::to_string(&envelope) {
                            Ok(payload) => payload,
                            Err(err) => {
                                logger.warn(
                                    "Failed to serialize WSL request cancellation",
                                    Some(serde_json::json!({"error": err.to_string()})),
                                );
                                continue;
                            }
                        };
                        if let Err(err) =
                            send_bounded(&mut sink, Message::Text(payload), limits.send).await
                        {
                            fail_pending_requests(
                                &mut pending,
                                format!("Failed to cancel WSL backend request: {err}"),
                            );
                            break;
                        }
                    }
                }
            }
            message = read_stream.next() => {
                let Some(message) = message else {
                    fail_pending_requests(&mut pending, "WSL backend disconnected");
                    break;
                };
                if message.is_ok() {
                    silence.as_mut().reset(Instant::now() + limits.silence);
                }
                match message {
                    Ok(Message::Text(text)) => {
                        handle_backend_message(
                            &text,
                            &mut pending,
                            event_bus,
                            logger,
                            generation,
                            outstanding_mutations,
                        );
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
    pending: &mut HashMap<String, oneshot::Sender<Result<ForwardedWslResponse, AgentError>>>,
    request_id: &str,
    err: String,
) {
    if let Some(response_tx) = pending.remove(request_id) {
        let _ = response_tx.send(Err(wsl_unavailable_error(format!(
            "Failed to send request to WSL backend: {err}"
        ))));
    }
    fail_pending_requests(pending, "WSL backend disconnected while sending request");
}
