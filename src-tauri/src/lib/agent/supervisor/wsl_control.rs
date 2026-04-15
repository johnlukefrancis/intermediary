// Path: src-tauri/src/lib/agent/supervisor/wsl_control.rs
// Description: WSL backend termination, stale-port remediation, and launch-target bookkeeping

use super::wsl_mode::WslBackendMode;
use super::wsl_runtime::{WSL_STALE_RETRY_BACKOFF, WSL_TERMINATE_POLL, WSL_TERMINATE_TERM_GRACE};
use super::AgentSupervisor;
use crate::agent::supervisor::state::ProcessKind;
use crate::agent::wsl_process_control::{
    format_wsl_target, list_exact_wsl_agent_pids, terminate_wsl_agent_process, WslLaunchTarget,
    WslTerminateOutcome,
};
use crate::obs::logging;

impl AgentSupervisor {
    pub(super) async fn terminate_wsl_backend_for_reason(
        &self,
        reason: &str,
    ) -> Result<(), String> {
        let Some(target) = self.wsl_launch_target_snapshot()? else {
            return Ok(());
        };
        self.terminate_wsl_backend_target(&target, reason, 1).await
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

    pub(super) async fn remediate_stale_wsl_port(
        &self,
        wsl_port: u16,
        target: &WslLaunchTarget,
    ) -> Result<(), String> {
        for attempt in 1..=2 {
            let reason = if attempt == 1 {
                "auth_probe_failed"
            } else {
                "auth_probe_failed_retry"
            };

            self.reconcile_recorded_child(ProcessKind::Wsl, reason)
                .await?;
            self.terminate_wsl_backend_target(target, reason, attempt)
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

        Err(format!(
            "WSL agent port {wsl_port} is occupied by a process that rejected the current websocket token ({})",
            format_wsl_target(target)
        ))
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
