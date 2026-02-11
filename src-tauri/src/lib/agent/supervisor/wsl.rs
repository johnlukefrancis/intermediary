// Path: src-tauri/src/lib/agent/supervisor/wsl.rs
// Description: WSL backend lifecycle helpers, including stale-port remediation and in-WSL termination

use super::{AgentSupervisor, EnsureProcessResult};
use crate::agent::install::AgentBundlePaths;
use crate::agent::process_control::wait_for_agent_ready;
use crate::agent::supervisor_helpers::{
    ProcessKind, WSL_STALE_RETRY_BACKOFF, WSL_TERMINATE_POLL, WSL_TERMINATE_TERM_GRACE,
};
use crate::agent::types::AgentWebSocketAuth;
use crate::agent::wsl_process_control::{
    build_wsl_launch_target, format_wsl_target, spawn_wsl_agent_process,
    terminate_wsl_agent_process, WslLaunchTarget, WslTerminateOutcome,
};
use crate::obs::logging;
use std::process::Child;

impl AgentSupervisor {
    pub(super) async fn ensure_wsl_running(
        &self,
        bundle: &AgentBundlePaths,
        distro: Option<&str>,
        wsl_port: u16,
        auth: &AgentWebSocketAuth,
        force: bool,
    ) -> Result<EnsureProcessResult, String> {
        let target = build_wsl_launch_target(bundle, distro)?;
        self.set_wsl_launch_target(Some(target.clone()))?;

        if self.probe_listening(wsl_port).await? {
            if self
                .probe_websocket_auth(wsl_port, &auth.wsl_ws_token)
                .await?
            {
                return Ok(EnsureProcessResult::AlreadyRunning);
            }
            self.remediate_stale_wsl_port(wsl_port, &target).await?;
        }

        self.reconcile_recorded_child(ProcessKind::Wsl, "port_probe_failed")
            .await?;
        if !force && self.is_in_backoff(ProcessKind::Wsl)? {
            return Ok(EnsureProcessResult::Backoff);
        }

        let target_summary = format_wsl_target(&target);
        logging::log(
            "info",
            "agent",
            "spawn_start",
            &format!("kind=wsl port={wsl_port} {target_summary}"),
        );
        let bundle_for_spawn = bundle.clone();
        let target_for_spawn = target.clone();
        let wsl_ws_token = auth.wsl_ws_token.clone();
        let spawned = tauri::async_runtime::spawn_blocking(move || -> Result<Child, String> {
            let mut child = spawn_wsl_agent_process(
                &bundle_for_spawn,
                &target_for_spawn,
                wsl_port,
                &wsl_ws_token,
            )?;
            wait_for_agent_ready(&mut child, wsl_port, ProcessKind::Wsl.label(), true)?;
            Ok(child)
        })
        .await
        .map_err(|err| format!("WSL agent spawn task failed: {err}"))?;

        match spawned {
            Ok(child) => {
                let pid = child.id();
                self.replace_child(ProcessKind::Wsl, child).await?;
                self.update_last_spawn(ProcessKind::Wsl)?;
                logging::log(
                    "info",
                    "agent",
                    "spawn_ready",
                    &format!("kind=wsl port={wsl_port} pid={pid} {target_summary}"),
                );
                Ok(EnsureProcessResult::Started)
            }
            Err(err) => {
                logging::log(
                    "error",
                    "agent",
                    "spawn_exit_early",
                    &format!("kind=wsl port={wsl_port} {target_summary} error={err}"),
                );
                self.set_last_error(Some(err.clone()))?;
                Err(err)
            }
        }
    }

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

    async fn remediate_stale_wsl_port(
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

    fn wsl_launch_target_snapshot(&self) -> Result<Option<WslLaunchTarget>, String> {
        let state = self
            .state
            .lock()
            .map_err(|_| "Agent supervisor lock poisoned".to_string())?;
        Ok(state.wsl_launch_target.clone())
    }

    async fn sleep_wsl_stale_retry_backoff(&self) -> Result<(), String> {
        tauri::async_runtime::spawn_blocking(move || std::thread::sleep(WSL_STALE_RETRY_BACKOFF))
            .await
            .map_err(|err| format!("WSL stale retry backoff task failed: {err}"))?;
        Ok(())
    }
}
