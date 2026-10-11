// Path: crates/im_host_agent/src/wsl/wsl_transport_status.rs
// Description: Generation-scoped wslBackendStatus online announcements and de-noised WSL transport error events

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use im_agent::error::AgentError;
use im_agent::protocol::{
    AgentErrorDetails, AgentErrorEvent, AgentEvent, WslBackendConnectionStatus,
    WslBackendStatusEvent,
};
use im_agent::server::EventBus;

use crate::error_codes::{WSL_BACKEND_TIMEOUT, WSL_BACKEND_UNAVAILABLE};

#[derive(Debug, Default)]
struct EpochState {
    offline_error_generation: Option<u64>,
}

impl EpochState {
    fn mark_success(&mut self, generation: u64) -> bool {
        if self.offline_error_generation == Some(generation) {
            self.offline_error_generation = None;
            return true;
        }
        false
    }

    fn should_emit_offline_error(&mut self, generation: u64) -> bool {
        if self.offline_error_generation == Some(generation) {
            return false;
        }
        self.offline_error_generation = Some(generation);
        true
    }
}

/// Owned by the WSL client so status events never need the host runtime lock.
#[derive(Clone)]
pub(crate) struct WslTransportStatus {
    event_bus: EventBus,
    generation: Arc<AtomicU64>,
    epoch: Arc<Mutex<EpochState>>,
}

impl WslTransportStatus {
    pub fn new(event_bus: EventBus, generation: Arc<AtomicU64>) -> Self {
        Self {
            event_bus,
            generation,
            epoch: Arc::new(Mutex::new(EpochState::default())),
        }
    }

    /// The backend is usable on `generation`: its hello applied, or none is needed.
    pub fn announce_online(&self, generation: u64) {
        if let Ok(mut epoch) = self.epoch.lock() {
            epoch.mark_success(generation);
        }
        self.emit_status(WslBackendConnectionStatus::Online, generation);
    }

    pub fn announce_offline(&self, generation: u64) {
        self.emit_status(WslBackendConnectionStatus::Offline, generation);
    }

    /// A forwarded command succeeded; announces recovery only after an offline error on that generation.
    pub fn mark_success(&self, generation: u64) {
        let recovered = self
            .epoch
            .lock()
            .map(|mut epoch| epoch.mark_success(generation))
            .unwrap_or(false);
        if recovered {
            self.emit_status(WslBackendConnectionStatus::Online, generation);
        }
    }

    pub fn report_transport_error(&self, err: &AgentError, repo_id: Option<String>) {
        let generation = self.generation.load(Ordering::SeqCst);
        self.report_transport_error_for_generation(err, repo_id, generation);
    }

    pub fn report_transport_error_for_generation(
        &self,
        err: &AgentError,
        repo_id: Option<String>,
        generation: u64,
    ) {
        if !is_wsl_transport_error(err) {
            return;
        }
        let emit = self
            .epoch
            .lock()
            .map(|mut epoch| epoch.should_emit_offline_error(generation))
            .unwrap_or(true);
        if emit {
            self.report_backend_error(err, repo_id);
        }
    }

    pub fn report_backend_error(&self, err: &AgentError, repo_id: Option<String>) {
        let message = err.message().to_string();
        let event = AgentErrorEvent::new(
            "wslBackend",
            message.clone(),
            Some(AgentErrorDetails {
                code: None,
                doc_path: None,
                repo_id,
                root_path: None,
                raw_code: Some(err.code().to_string()),
                raw_message: Some(message),
            }),
        );
        self.event_bus.broadcast_event(AgentEvent::Error(event));
    }

    fn emit_status(&self, status: WslBackendConnectionStatus, generation: u64) {
        self.event_bus
            .broadcast_event(AgentEvent::WslBackendStatus(WslBackendStatusEvent {
                status,
                generation,
            }));
    }
}

pub(crate) fn is_wsl_transport_error(err: &AgentError) -> bool {
    err.code() == WSL_BACKEND_UNAVAILABLE || err.code() == WSL_BACKEND_TIMEOUT
}

#[cfg(test)]
#[path = "wsl_transport_status_tests.rs"]
pub(crate) mod tests;
