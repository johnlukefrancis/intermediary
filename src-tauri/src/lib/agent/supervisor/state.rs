// Path: src-tauri/src/lib/agent/supervisor/state.rs
// Description: Shared supervisor process state and process-kind labels

use crate::agent::wsl_process_control::WslLaunchTarget;
use std::process::Child;
use std::time::Instant;

#[derive(Debug, Clone, Copy)]
pub(super) enum ProcessKind {
    Host,
    Wsl,
}

impl ProcessKind {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Host => "Host agent",
            Self::Wsl => "WSL agent",
        }
    }

    pub(super) fn log_key(self) -> &'static str {
        match self {
            Self::Host => "host",
            Self::Wsl => "wsl",
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct ManagedProcessState {
    pub child: Option<Child>,
    pub last_spawn_at: Option<Instant>,
}

#[derive(Debug, Default)]
pub(super) struct AgentSupervisorState {
    pub host: ManagedProcessState,
    pub wsl: ManagedProcessState,
    pub wsl_launch_target: Option<WslLaunchTarget>,
    pub last_error: Option<String>,
}

pub(super) fn process_state(
    state: &AgentSupervisorState,
    kind: ProcessKind,
) -> &ManagedProcessState {
    match kind {
        ProcessKind::Host => &state.host,
        ProcessKind::Wsl => &state.wsl,
    }
}

pub(super) fn process_state_mut(
    state: &mut AgentSupervisorState,
    kind: ProcessKind,
) -> &mut ManagedProcessState {
    match kind {
        ProcessKind::Host => &mut state.host,
        ProcessKind::Wsl => &mut state.wsl,
    }
}
