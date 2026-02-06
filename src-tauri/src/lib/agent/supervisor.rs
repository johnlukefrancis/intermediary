// Path: src-tauri/src/lib/agent/supervisor.rs
// Description: Public dual-agent supervisor types and wiring

use super::supervisor_helpers::AgentSupervisorState;
use super::types::{AgentSupervisorResult, AgentSupervisorStatus};
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
    pub(super) state: Mutex<AgentSupervisorState>,
}

fn build_result(
    status: AgentSupervisorStatus,
    port: u16,
    wsl_port: u16,
    requires_wsl: bool,
    agent_dir: String,
    log_dir: String,
    message: Option<String>,
) -> AgentSupervisorResult {
    AgentSupervisorResult {
        status,
        port,
        wsl_port,
        requires_wsl,
        agent_dir,
        log_dir,
        message,
    }
}

mod lifecycle;
mod processes;
