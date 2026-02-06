// Path: src-tauri/src/lib/agent/supervisor_helpers.rs
// Description: Shared state and helper utilities for dual-agent supervision

use std::process::Child;
use std::time::Instant;
use tauri::{AppHandle, Manager};

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

pub(super) fn resolve_wsl_port(host_port: u16, requires_wsl: bool) -> Result<u16, String> {
    if !requires_wsl {
        return Ok(host_port.saturating_add(1));
    }

    host_port
        .checked_add(1)
        .ok_or_else(|| "Agent port 65535 cannot reserve WSL backend port".to_string())
}

pub(super) fn resolve_expected_dirs(app: &AppHandle) -> Result<(String, String), String> {
    let app_local_data = app
        .path()
        .app_local_data_dir()
        .map_err(|_| "Failed to resolve app local data directory".to_string())?;
    let agent_dir = app_local_data.join("agent");
    let log_dir = app_local_data.join("logs");
    Ok((
        agent_dir.display().to_string(),
        log_dir.display().to_string(),
    ))
}

pub(super) fn should_prefer_installed_bundle(host_listening: bool, wsl_listening: bool) -> bool {
    host_listening || wsl_listening
}

#[cfg(test)]
mod tests {
    use super::{resolve_wsl_port, should_prefer_installed_bundle};

    #[test]
    fn resolve_wsl_port_for_wsl_repos_uses_next_port() {
        assert_eq!(resolve_wsl_port(3141, true).expect("port"), 3142);
    }

    #[test]
    fn resolve_wsl_port_for_windows_only_allows_max_host_port() {
        assert_eq!(resolve_wsl_port(u16::MAX, false).expect("port"), u16::MAX);
    }

    #[test]
    fn resolve_wsl_port_for_wsl_repos_rejects_u16_overflow() {
        let error = resolve_wsl_port(u16::MAX, true).expect_err("expected overflow");
        assert!(
            error.contains("cannot reserve WSL backend port"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn installed_bundle_is_preferred_when_host_or_wsl_is_alive() {
        assert!(should_prefer_installed_bundle(true, false));
        assert!(should_prefer_installed_bundle(false, true));
        assert!(should_prefer_installed_bundle(true, true));
        assert!(!should_prefer_installed_bundle(false, false));
    }
}
