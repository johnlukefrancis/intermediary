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

mod graceful_stop;
mod host;
mod lifecycle;
mod managed_processes;
mod probes;
mod process_kill;
mod runtime;
mod shutdown;
mod shutdown_ws_client;
mod state;
mod websocket_frame;
mod websocket_probe;
mod wsl;
mod wsl_control;
mod wsl_logging;
mod wsl_mode;
mod wsl_runtime;
mod wsl_same_port_termination;
