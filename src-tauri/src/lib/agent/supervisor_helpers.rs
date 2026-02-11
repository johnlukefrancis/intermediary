// Path: src-tauri/src/lib/agent/supervisor_helpers.rs
// Description: Shared state and helper utilities for host-agent supervision with optional Windows WSL backend

use std::process::Child;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

pub(super) const KILL_WAIT_TIMEOUT: Duration = Duration::from_secs(5);
pub(super) const KILL_WAIT_POLL: Duration = Duration::from_millis(50);

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

pub(super) enum KillAndWaitOutcome {
    Exited(String),
    Failed(Child, String),
}

pub(super) fn kill_and_wait(mut child: Child) -> KillAndWaitOutcome {
    if let Err(err) = child.kill() {
        match child.try_wait() {
            Ok(Some(status)) => return KillAndWaitOutcome::Exited(status.to_string()),
            Ok(None) => {
                return KillAndWaitOutcome::Failed(child, format!("kill signal failed: {err}"));
            }
            Err(wait_err) => {
                return KillAndWaitOutcome::Failed(
                    child,
                    format!("kill signal failed: {err}; poll failed: {wait_err}"),
                );
            }
        }
    }

    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return KillAndWaitOutcome::Exited(status.to_string()),
            Ok(None) => {
                if start.elapsed() >= KILL_WAIT_TIMEOUT {
                    return KillAndWaitOutcome::Failed(
                        child,
                        format!(
                            "process did not exit within {}ms after kill",
                            KILL_WAIT_TIMEOUT.as_millis()
                        ),
                    );
                }
                thread::sleep(KILL_WAIT_POLL);
            }
            Err(err) => return KillAndWaitOutcome::Failed(child, err.to_string()),
        }
    }
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
