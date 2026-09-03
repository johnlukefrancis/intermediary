// Path: src-tauri/src/lib/agent/supervisor/wsl_backend_record.rs
// Description: What the supervisor records about the WSL backend it owns this session

use super::AgentSupervisor;
use crate::agent::supervisor::state::WslBackendHandle;
use crate::agent::wsl_process_control::WslLaunchTarget;

impl AgentSupervisor {
    pub(super) fn set_wsl_launch_target(
        &self,
        target: Option<WslLaunchTarget>,
    ) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Agent supervisor lock poisoned".to_string())?;
        state.wsl_launch_target = target;
        Ok(())
    }
    /// Commits this backend as supervisor-owned: records both the exact-path kill target and the
    /// durable (distro, port) handle used by config-less reclamation. Call ONLY once ownership is
    /// confirmed reclaimable — adopting a healthy backend, remediating our own occupant, or right
    /// before a managed spawn — never for a foreign/ExternalUnmanaged occupant, whose distro must
    /// not be torn down on app exit (ADR-013 boundary).
    pub(super) fn record_owned_wsl_backend(
        &self,
        target: &WslLaunchTarget,
        wsl_port: u16,
    ) -> Result<(), String> {
        self.set_wsl_launch_target(Some(target.clone()))?;
        self.set_last_wsl_backend(Some(WslBackendHandle {
            distro: target.distro.clone(),
            port: wsl_port,
        }))
    }

    pub(super) fn set_last_wsl_backend(
        &self,
        handle: Option<WslBackendHandle>,
    ) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Agent supervisor lock poisoned".to_string())?;
        state.last_wsl_backend = handle;
        Ok(())
    }

    pub(super) fn last_wsl_backend_snapshot(&self) -> Result<Option<WslBackendHandle>, String> {
        let state = self
            .state
            .lock()
            .map_err(|_| "Agent supervisor lock poisoned".to_string())?;
        Ok(state.last_wsl_backend.clone())
    }
    pub(super) fn wsl_launch_target_snapshot(&self) -> Result<Option<WslLaunchTarget>, String> {
        let state = self
            .state
            .lock()
            .map_err(|_| "Agent supervisor lock poisoned".to_string())?;
        Ok(state.wsl_launch_target.clone())
    }
}
