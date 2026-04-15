// Path: src-tauri/src/lib/agent/supervisor.rs
// Description: Public host-agent supervisor types and wiring

use super::types::{AgentSupervisorResult, AgentSupervisorStatus, AgentSupervisorWslStatus};
use state::AgentSupervisorState;
use std::sync::Mutex;
use std::time::Duration;

const SPAWN_BACKOFF: Duration = Duration::from_millis(1500);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EnsureProcessResult {
    Started,
    AlreadyRunning,
    Backoff,
}

#[derive(Debug, Default)]
pub struct AgentSupervisor {
    state: Mutex<AgentSupervisorState>,
}

fn build_result(
    status: AgentSupervisorStatus,
    port: u16,
    supports_wsl: bool,
    wsl: Option<AgentSupervisorWslStatus>,
    agent_dir: String,
    log_dir: String,
    message: Option<String>,
) -> AgentSupervisorResult {
    AgentSupervisorResult {
        status,
        port,
        supports_wsl,
        wsl,
        agent_dir,
        log_dir,
        message,
    }
}

mod host;
mod lifecycle;
mod managed_processes;
mod probes;
mod process_kill;
mod runtime;
mod state;
mod websocket_probe;
mod wsl;
mod wsl_control;
mod wsl_mode;
mod wsl_runtime;
