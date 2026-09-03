// Path: crates/im_host_agent/src/wsl/wsl_backend_client.rs
// Description: Persistent WebSocket client for forwarding commands/events to the WSL backend agent
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::error_codes::WSL_BACKEND_TIMEOUT;
use im_agent::error::AgentError;
use im_agent::logging::Logger;
use im_agent::protocol::{UiCommand, UiResponse};
use im_agent::server::EventBus;
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;

use super::wsl_backend_messages::wsl_unavailable_error;
use client_loop::run_client_loop;
use timeouts::timeout_for_command;

mod client_loop;
mod timeouts;

/// Request ids of forwarded mutations this client has sent to the WSL agent
/// and not had confirmed back. Shared with the request loop, which is the one
/// place that knows a request never reached the wire.
pub(super) type OutstandingMutations = Arc<Mutex<HashSet<String>>>;

#[derive(Clone)]
pub struct WslBackendClient {
    request_tx: mpsc::UnboundedSender<RequestLoopMessage>,
    request_counter: Arc<AtomicU64>,
    connection_generation: Arc<AtomicU64>,
    /// Request ids of forwarded `sourceControlAction` commands this client has
    /// sent to the WSL agent but never received a confirmed response for. A
    /// shutdown drain reads this to tell "the connection just hiccupped" from
    /// "a mutation may still be running over there" when the backend answers
    /// `WSL_BACKEND_UNAVAILABLE`.
    outstanding_mutations: OutstandingMutations,
}

#[derive(Debug)]
pub struct ForwardedWslResponse {
    pub response: UiResponse,
    pub generation: u64,
}

pub(crate) enum RequestLoopMessage {
    Forward(Box<ForwardRequest>),
    Cancel { request_id: String },
}

pub(crate) struct ForwardRequest {
    pub(crate) request_id: String,
    pub(crate) command: UiCommand,
    pub(crate) response_tx: oneshot::Sender<Result<ForwardedWslResponse, AgentError>>,
}
impl WslBackendClient {
    pub fn new(wsl_port: u16, wsl_ws_token: String, event_bus: EventBus, logger: Logger) -> Self {
        let (request_tx, request_rx) = mpsc::unbounded_channel();
        let endpoint_log = format!("ws://127.0.0.1:{wsl_port}");
        let endpoint_connect = format!("{endpoint_log}/?token={wsl_ws_token}");
        let connection_generation = Arc::new(AtomicU64::new(0));
        let outstanding_mutations: OutstandingMutations = Arc::new(Mutex::new(HashSet::new()));

        tokio::spawn(run_client_loop(
            endpoint_connect,
            endpoint_log,
            request_rx,
            event_bus,
            logger,
            connection_generation.clone(),
            outstanding_mutations.clone(),
        ));

        Self {
            request_tx,
            request_counter: Arc::new(AtomicU64::new(0)),
            connection_generation,
            outstanding_mutations,
        }
    }

    /// A client whose request loop is the caller's own channel: a test drives
    /// each arm the loop would take itself, instead of racing a real
    /// connection for the one state that matters here — a mutation forwarded
    /// with no answer back.
    #[cfg(test)]
    pub(crate) fn with_request_channel() -> (Self, mpsc::UnboundedReceiver<RequestLoopMessage>) {
        let (request_tx, request_rx) = mpsc::unbounded_channel();
        let client = Self {
            request_tx,
            request_counter: Arc::new(AtomicU64::new(0)),
            connection_generation: Arc::new(AtomicU64::new(0)),
            outstanding_mutations: Arc::new(Mutex::new(HashSet::new())),
        };
        (client, request_rx)
    }
    pub fn connection_generation(&self) -> u64 {
        self.connection_generation.load(Ordering::SeqCst)
    }

    /// Whether a forwarded `sourceControlAction` request is still outstanding:
    /// sent to the WSL agent with no confirmed response back yet. A shutdown
    /// drain treats a non-empty set as proof it must not trust
    /// `WSL_BACKEND_UNAVAILABLE` as idle.
    pub fn has_outstanding_mutations(&self) -> bool {
        self.outstanding_mutation_count() > 0
    }

    /// How many forwarded mutations are outstanding right now; the residue a
    /// shutdown drain reports when it gives up at the emergency bound.
    pub fn outstanding_mutation_count(&self) -> u32 {
        let count = self
            .outstanding_mutations
            .lock()
            .map(|set| set.len())
            .unwrap_or(0);
        u32::try_from(count).unwrap_or(u32::MAX)
    }

    pub async fn forward_command_with_generation(
        &self,
        command: UiCommand,
    ) -> Result<ForwardedWslResponse, AgentError> {
        let timeout_duration = timeout_for_command(&command);
        self.forward_command_with_timeout(command, timeout_duration)
            .await
    }

    /// Forwards one command with an explicit timeout, so a shutdown drain can
    /// bound each retry by its own remaining budget rather than the fixed
    /// per-command ladder in `timeouts.rs`.
    pub(crate) async fn forward_command_with_timeout(
        &self,
        command: UiCommand,
        timeout_duration: Duration,
    ) -> Result<ForwardedWslResponse, AgentError> {
        let request_id = self.next_request_id();
        // Only a mutation leaves residue Git cares about; reads and the
        // shutdown command itself are never tracked here.
        let is_mutation = matches!(command, UiCommand::SourceControlAction(_));
        if is_mutation {
            self.track_outstanding(&request_id);
        }
        let (response_tx, response_rx) = oneshot::channel();

        if self
            .request_tx
            .send(RequestLoopMessage::Forward(Box::new(ForwardRequest {
                request_id: request_id.clone(),
                command,
                response_tx,
            })))
            .is_err()
        {
            // Never left the host's own queue, so nothing is outstanding at
            // the backend for it.
            self.untrack_outstanding(&request_id);
            return Err(wsl_unavailable_error("WSL backend request loop is offline"));
        }

        let timeout_ms = timeout_duration.as_millis();
        let outcome = match timeout(timeout_duration, response_rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(wsl_unavailable_error(
                "WSL backend closed before returning a response",
            )),
            Err(_) => {
                let _ = self.request_tx.send(RequestLoopMessage::Cancel {
                    request_id: request_id.clone(),
                });
                Err(AgentError::new(
                    WSL_BACKEND_TIMEOUT,
                    format!("WSL backend timed out after {timeout_ms}ms waiting for response"),
                ))
            }
        };
        // Only a confirmed round trip proves the mutation is done, whatever
        // its own outcome; every other exit here leaves it tracked so a
        // shutdown drain keeps waiting rather than trusting a transport
        // failure to mean the Git process on the other side is gone too.
        if is_mutation && outcome.is_ok() {
            self.untrack_outstanding(&request_id);
        }
        outcome
    }

    fn track_outstanding(&self, request_id: &str) {
        if let Ok(mut set) = self.outstanding_mutations.lock() {
            set.insert(request_id.to_string());
        }
    }

    fn untrack_outstanding(&self, request_id: &str) {
        untrack_outstanding(&self.outstanding_mutations, request_id);
    }

    fn next_request_id(&self) -> String {
        let next = self.request_counter.fetch_add(1, Ordering::Relaxed) + 1;
        format!("host_wsl_req_{next}")
    }
}
/// A forwarded request stops being outstanding: it is either confirmed done
/// or proven never to have reached the WSL agent at all.
pub(super) fn untrack_outstanding(outstanding: &OutstandingMutations, request_id: &str) {
    if let Ok(mut set) = outstanding.lock() {
        set.remove(request_id);
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_timeouts;
