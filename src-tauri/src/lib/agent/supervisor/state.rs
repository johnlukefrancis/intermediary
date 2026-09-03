// Path: src-tauri/src/lib/agent/supervisor/state.rs
// Description: Shared supervisor process state and process-kind labels

use super::graceful_stop::GracefulStopPath;
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

/// Durable identity of the WSL backend we manage this session (distro + reserved port). Unlike
/// `wsl_launch_target` — which is cleared and re-derived on every ensure pass — this survives so
/// config-less callers (`stop`, app exit) can reclaim the backend by port even when they hold no
/// launch target (adopted/reconnected backend, or a health-check race).
#[derive(Debug, Clone)]
pub(super) struct WslBackendHandle {
    pub distro: Option<String>,
    pub port: u16,
}

/// Durable identity of the host agent this session owns: the port it serves and
/// the token that authenticates us to it. `stop`, `restart`, and app exit carry
/// no config, and a graceful shutdown has to reach the same socket the
/// supervisor started or adopted.
#[derive(Debug, Clone)]
pub(super) struct HostBackendHandle {
    pub port: u16,
    pub ws_token: String,
}

#[derive(Debug, Default)]
pub(super) struct AgentSupervisorState {
    pub host: ManagedProcessState,
    pub wsl: ManagedProcessState,
    pub wsl_launch_target: Option<WslLaunchTarget>,
    pub last_host_backend: Option<HostBackendHandle>,
    pub last_wsl_backend: Option<WslBackendHandle>,
    pub last_error: Option<String>,
    /// How the host agent's most recent stop actually ended
    /// (`graceful_stop::stop_host_gracefully`). App-exit teardown reads this
    /// to decide whether the WSL distro is safe to terminate: never while
    /// finality came back `Unknown`.
    pub last_host_stop_finality: Option<GracefulStopPath>,
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
