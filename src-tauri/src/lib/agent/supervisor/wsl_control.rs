// Path: src-tauri/src/lib/agent/supervisor/wsl_control.rs
// Description: WSL backend termination, stale-port remediation, and launch-target bookkeeping

use super::wsl_mode::{WslBackendMode, WslBackendOwner};
use super::wsl_runtime::{WSL_STALE_RETRY_BACKOFF, WSL_TERMINATE_POLL, WSL_TERMINATE_TERM_GRACE};
use super::AgentSupervisor;
use crate::agent::supervisor::state::{ProcessKind, WslBackendHandle};
use crate::agent::wsl_process_control::{
    format_wsl_target, list_exact_wsl_agent_pids, list_reclaimable_wsl_agent_pids_by_port,
    terminate_wsl_agent_by_port_listener, terminate_wsl_agent_process, WslLaunchTarget,
    WslTerminateOutcome,
};
use crate::obs::logging;

impl AgentSupervisor {
    pub(super) async fn terminate_wsl_backend_for_reason(
        &self,
        reason: &str,
    ) -> Result<(), String> {
        let mut errors: Vec<String> = Vec::new();

        // Precise kill by the exact configured binary path when we recorded a launch target.
        if let Some(target) = self.wsl_launch_target_snapshot()? {
            if let Err(err) = self.terminate_wsl_backend_target(&target, reason, 1).await {
                errors.push(err);
            }
        }

        // Robust port reclamation: kill any Intermediary im_agent still bound to our backend port,
        // even one this session did not spawn (adopted / stale / token-mismatched). This is what
        // makes `stop`, Restart Agent, and app exit reliable instead of no-ops.
        if let Some(handle) = self.last_wsl_backend_snapshot()? {
            if let Err(err) = self
                .reclaim_wsl_backend_by_port(handle.distro.as_deref(), handle.port, reason)
                .await
            {
                errors.push(err);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }

    async fn reclaim_wsl_backend_by_port(
        &self,
        distro: Option<&str>,
        wsl_port: u16,
        reason: &str,
    ) -> Result<(), String> {
        let distro_owned = distro.map(str::to_string);
        let result = tauri::async_runtime::spawn_blocking(move || {
            terminate_wsl_agent_by_port_listener(
                distro_owned.as_deref(),
                wsl_port,
                WSL_TERMINATE_TERM_GRACE,
                WSL_TERMINATE_POLL,
            )
        })
        .await
        .map_err(|err| format!("WSL port reclamation task failed: {err}"))?;

        let distro_label = distro.unwrap_or("default");
        match result {
            Ok(outcome) => {
                let outcome_label = match outcome {
                    WslTerminateOutcome::NoMatch => "no_match",
                    WslTerminateOutcome::TerminatedWithTerm => "term",
                    WslTerminateOutcome::TerminatedWithKill => "kill",
                };
                logging::log(
                    "info",
                    "agent",
                    "wsl_port_reclaim_done",
                    &format!(
                        "reason={reason} port={wsl_port} outcome={outcome_label} distro={distro_label}"
                    ),
                );
                Ok(())
            }
            Err(err) => {
                logging::log(
                    "warn",
                    "agent",
                    "wsl_port_reclaim_done",
                    &format!(
                        "reason={reason} port={wsl_port} outcome=failed distro={distro_label} error={err}"
                    ),
                );
                Err(err)
            }
        }
    }

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

    pub(super) async fn remediate_stale_wsl_port(
        &self,
        wsl_port: u16,
        target: &WslLaunchTarget,
        owner: WslBackendOwner,
        reason: &str,
    ) -> Result<(), String> {
        for attempt in 1..=2 {
            let attempt_reason = if attempt == 1 {
                reason.to_string()
            } else {
                format!("{reason}_retry")
            };

            self.reconcile_recorded_child(ProcessKind::Wsl, &attempt_reason)
                .await?;
            self.terminate_wsl_backend_target_for_owner(
                target,
                wsl_port,
                owner,
                &attempt_reason,
                attempt,
            )
            .await?;
            if !self.probe_listening(wsl_port).await? {
                return Ok(());
            }

            if attempt == 1 {
                logging::log(
                    "info",
                    "agent",
                    "stale_retry_wait",
                    &format!(
                        "kind=wsl port={wsl_port} wait_ms={} {}",
                        WSL_STALE_RETRY_BACKOFF.as_millis(),
                        format_wsl_target(target)
                    ),
                );
                self.sleep_wsl_stale_retry_backoff().await?;
            }
        }

        Err(format_stale_wsl_port_error(wsl_port, target, owner, reason))
    }

    pub(super) async fn detect_installed_wsl_pid_count(
        &self,
        target: &WslLaunchTarget,
    ) -> Result<usize, String> {
        let target_for_probe = target.clone();
        tauri::async_runtime::spawn_blocking(move || list_exact_wsl_agent_pids(&target_for_probe))
            .await
            .map_err(|err| format!("WSL owner-detection task failed: {err}"))?
            .map(|pids| pids.len())
    }

    pub(super) async fn detect_intermediary_wsl_pid_count_by_port(
        &self,
        target: &WslLaunchTarget,
        wsl_port: u16,
    ) -> Result<usize, String> {
        let target_for_probe = target.clone();
        tauri::async_runtime::spawn_blocking(move || {
            list_reclaimable_wsl_agent_pids_by_port(&target_for_probe, wsl_port)
        })
        .await
        .map_err(|err| format!("WSL same-port owner-detection task failed: {err}"))?
        .map(|pids| pids.len())
    }

    pub(super) fn wsl_launch_target_snapshot(&self) -> Result<Option<WslLaunchTarget>, String> {
        let state = self
            .state
            .lock()
            .map_err(|_| "Agent supervisor lock poisoned".to_string())?;
        Ok(state.wsl_launch_target.clone())
    }

    async fn terminate_wsl_backend_target(
        &self,
        target: &WslLaunchTarget,
        reason: &str,
        attempt: usize,
    ) -> Result<(), String> {
        let target_summary = format_wsl_target(target);
        logging::log(
            "info",
            "agent",
            "wsl_terminate_start",
            &format!("reason={reason} attempt={attempt} {target_summary}"),
        );
        let target_for_kill = target.clone();
        let result = tauri::async_runtime::spawn_blocking(move || {
            terminate_wsl_agent_process(
                &target_for_kill,
                WSL_TERMINATE_TERM_GRACE,
                WSL_TERMINATE_POLL,
            )
        })
        .await
        .map_err(|err| format!("WSL termination task failed: {err}"))?;

        match result {
            Ok(WslTerminateOutcome::NoMatch) => {
                logging::log(
                    "info",
                    "agent",
                    "wsl_terminate_done",
                    &format!("reason={reason} attempt={attempt} outcome=no_match {target_summary}"),
                );
                Ok(())
            }
            Ok(WslTerminateOutcome::TerminatedWithTerm) => {
                logging::log(
                    "info",
                    "agent",
                    "wsl_terminate_done",
                    &format!("reason={reason} attempt={attempt} outcome=term {target_summary}"),
                );
                Ok(())
            }
            Ok(WslTerminateOutcome::TerminatedWithKill) => {
                logging::log(
                    "warn",
                    "agent",
                    "wsl_terminate_done",
                    &format!("reason={reason} attempt={attempt} outcome=kill {target_summary}"),
                );
                Ok(())
            }
            Err(err) => {
                logging::log(
                    "error",
                    "agent",
                    "wsl_terminate_done",
                    &format!(
                        "reason={reason} attempt={attempt} outcome=failed {target_summary} error={err}"
                    ),
                );
                Err(err)
            }
        }
    }

    async fn terminate_wsl_backend_target_for_owner(
        &self,
        target: &WslLaunchTarget,
        wsl_port: u16,
        owner: WslBackendOwner,
        reason: &str,
        attempt: usize,
    ) -> Result<(), String> {
        match owner {
            WslBackendOwner::SamePortIntermediary => {
                self.terminate_wsl_backend_same_port_intermediary(target, wsl_port, reason, attempt)
                    .await
            }
            WslBackendOwner::InstalledManaged | WslBackendOwner::ExternalUnmanaged => {
                self.terminate_wsl_backend_target(target, reason, attempt)
                    .await
            }
        }
    }

    async fn sleep_wsl_stale_retry_backoff(&self) -> Result<(), String> {
        tauri::async_runtime::spawn_blocking(move || std::thread::sleep(WSL_STALE_RETRY_BACKOFF))
            .await
            .map_err(|err| format!("WSL stale retry backoff task failed: {err}"))?;
        Ok(())
    }

    pub(super) fn managed_mode_error_for_external_occupant(
        mode: WslBackendMode,
        wsl_port: u16,
        target: &WslLaunchTarget,
    ) -> Option<String> {
        if !matches!(mode, WslBackendMode::Managed) {
            return None;
        }

        Some(format!(
            "WSL backend mode=managed expected the installed backend process, but an external occupant is using port {wsl_port} ({})",
            format_wsl_target(target)
        ))
    }
}

fn format_stale_wsl_port_error(
    wsl_port: u16,
    target: &WslLaunchTarget,
    owner: WslBackendOwner,
    reason: &str,
) -> String {
    if reason == "managed_owner_mismatch" {
        return format!(
            "WSL agent port {wsl_port} is occupied by owner={} instead of the configured installed backend ({})",
            owner.log_key(),
            format_wsl_target(target)
        );
    }

    format!(
        "WSL agent port {wsl_port} is occupied by owner={} that rejected the current websocket token ({})",
        owner.log_key(),
        format_wsl_target(target)
    )
}
