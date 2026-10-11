// Path: crates/im_host_agent/src/wsl/wsl_backend_client/hello_reconciler.rs
// Description: The single in-flight WSL clientHello reconciliation per connection generation, owned by the WSL client

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

use im_agent::error::AgentError;
use im_agent::logging::Logger;
use im_agent::protocol::{ClientHelloCommand, UiCommand, UiResponse};
use tokio::sync::{mpsc, watch, Notify};

use super::super::wsl_transport_status::{is_wsl_transport_error, WslTransportStatus};
use super::hello_state::WslHelloState;
use super::timeouts::FORWARD_REQUEST_TIMEOUT_CLIENT_HELLO;
use super::{OutstandingMutations, RequestLoopMessage, WslBackendClient};
use crate::error_codes::{WSL_BACKEND_UNAVAILABLE, WSL_BACKEND_VERSION_MISMATCH};

const ROOT_IDENTITY_MIN_AGENT_VERSION: &str = "0.1.24";

/// Hello state shared by the client, its waiters and the reconciler task.
pub(crate) struct WslHelloShared {
    state: Mutex<WslHelloState>,
    wake: Notify,
    changed: watch::Sender<u64>,
    closed: AtomicBool,
}

impl WslHelloShared {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(WslHelloState::default()),
            wake: Notify::new(),
            changed: watch::channel(0).0,
            closed: AtomicBool::new(false),
        })
    }

    pub(super) fn state(&self) -> MutexGuard<'_, WslHelloState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn set_desired(&self, command: Option<ClientHelloCommand>) -> Result<(), AgentError> {
        if self.state().set_desired(command)? {
            self.kick();
        }
        Ok(())
    }

    pub fn has_desired(&self) -> bool {
        self.state().has_desired()
    }

    /// A new connection generation needs its own hello; waiters re-evaluate against it.
    pub fn note_connected(&self) {
        self.kick();
    }

    /// The request loop is gone: the reconciler task exits on its next wake.
    pub fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
        self.kick();
    }

    pub(super) fn kick(&self) {
        self.wake.notify_one();
        self.publish();
    }

    /// Wakes the task without publishing, so the waiter that calls it does not wake itself.
    pub(super) fn wake_reconciler(&self) {
        self.wake.notify_one();
    }

    pub(super) fn subscribe(&self) -> watch::Receiver<u64> {
        self.changed.subscribe()
    }

    fn publish(&self) {
        self.changed
            .send_modify(|version| *version = version.wrapping_add(1));
    }
}

/// Opens a connection generation: online now when no hello is needed, else once its hello applies.
pub(super) fn begin_generation(
    connection_generation: &AtomicU64,
    hello: &WslHelloShared,
    status: &WslTransportStatus,
) -> u64 {
    let generation = connection_generation.fetch_add(1, Ordering::SeqCst) + 1;
    if !hello.has_desired() {
        status.announce_online(generation);
    }
    hello.note_connected();
    generation
}

/// Holds no live sender, so the request loop still ends when the last client drops.
pub(super) struct WeakWslBackendClient {
    request_tx: mpsc::WeakUnboundedSender<RequestLoopMessage>,
    request_counter: Arc<AtomicU64>,
    connection_generation: Arc<AtomicU64>,
    outstanding_mutations: OutstandingMutations,
    hello: Arc<WslHelloShared>,
    status: WslTransportStatus,
}

impl WeakWslBackendClient {
    pub fn new(client: &WslBackendClient) -> Self {
        Self {
            request_tx: client.request_tx.downgrade(),
            request_counter: client.request_counter.clone(),
            connection_generation: client.connection_generation.clone(),
            outstanding_mutations: client.outstanding_mutations.clone(),
            hello: client.hello.clone(),
            status: client.status.clone(),
        }
    }

    fn upgrade(&self) -> Option<WslBackendClient> {
        Some(WslBackendClient {
            request_tx: self.request_tx.upgrade()?,
            request_counter: self.request_counter.clone(),
            connection_generation: self.connection_generation.clone(),
            outstanding_mutations: self.outstanding_mutations.clone(),
            hello: self.hello.clone(),
            status: self.status.clone(),
        })
    }
}

pub(super) async fn run_hello_reconciler(weak: WeakWslBackendClient, logger: Logger) {
    loop {
        weak.hello.wake.notified().await;
        if weak.hello.closed.load(Ordering::SeqCst) {
            return;
        }
        let Some(client) = weak.upgrade() else {
            return;
        };
        reconcile(&client, &logger).await;
    }
}

/// Latest desired payload wins: loops until it is applied on the current generation or an attempt fails.
async fn reconcile(client: &WslBackendClient, logger: &Logger) {
    let hello = &client.hello;
    loop {
        let generation = client.connection_generation();
        let Some(attempt) = hello.state().begin_attempt(generation) else {
            return;
        };
        hello.publish();
        match send_hello(client, attempt.command, logger).await {
            Ok((watched, applied_generation)) => {
                let first =
                    hello
                        .state()
                        .finish_applied(&attempt.fingerprint, applied_generation, watched);
                hello.publish();
                if first && applied_generation == client.connection_generation() {
                    client.status.announce_online(applied_generation);
                }
            }
            Err(err) => {
                hello.state().finish_failed(&attempt.fingerprint, &err);
                hello.publish();
                if is_wsl_transport_error(&err) {
                    client.status.report_transport_error(&err, None);
                } else {
                    client.status.report_backend_error(&err, None);
                }
                return;
            }
        }
    }
}

async fn send_hello(
    client: &WslBackendClient,
    command: ClientHelloCommand,
    logger: &Logger,
) -> Result<(Vec<String>, u64), AgentError> {
    let started = Instant::now();
    let forwarded = client
        .forward_command_with_timeout(
            UiCommand::ClientHello(command),
            FORWARD_REQUEST_TIMEOUT_CLIENT_HELLO,
        )
        .await
        .and_then(|forwarded| match forwarded.response {
            UiResponse::ClientHelloResult(result) => {
                require_root_identity_version(&result.agent_version)?;
                Ok((result.watched_repo_ids, forwarded.generation))
            }
            _ => Err(AgentError::new(
                WSL_BACKEND_UNAVAILABLE,
                "WSL backend returned unexpected response type for clientHello",
            )),
        });
    let duration_ms = started.elapsed().as_millis();
    match &forwarded {
        Ok((watched, generation)) => logger.info(
            "WSL backend clientHello applied",
            Some(serde_json::json!({
                "durationMs": duration_ms,
                "generation": generation,
                "watchedRepoCount": watched.len()
            })),
        ),
        Err(err) => logger.warn(
            "WSL backend clientHello failed",
            Some(serde_json::json!({
                "code": err.code(),
                "durationMs": duration_ms,
                "generation": client.connection_generation()
            })),
        ),
    }
    forwarded
}

pub(super) fn require_root_identity_version(actual: &str) -> Result<(), AgentError> {
    if numeric_version(actual)
        .zip(numeric_version(ROOT_IDENTITY_MIN_AGENT_VERSION))
        .is_some_and(|(actual, required)| actual >= required)
    {
        return Ok(());
    }
    Err(AgentError::new(
        WSL_BACKEND_VERSION_MISMATCH,
        format!("WSL backend requires agent version {ROOT_IDENTITY_MIN_AGENT_VERSION} or newer for root identity; received {actual}"),
    ))
}

fn numeric_version(version: &str) -> Option<(u64, u64, u64)> {
    let mut parts = version.split('.');
    let mut number = || {
        let part = parts.next()?;
        if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        part.parse::<u64>().ok()
    };
    let triple = (number()?, number()?, number()?);
    parts.next().is_none().then_some(triple)
}
