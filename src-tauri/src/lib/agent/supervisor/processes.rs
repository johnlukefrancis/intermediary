// Path: src-tauri/src/lib/agent/supervisor/processes.rs
// Description: Process lifecycle helpers for host/WSL supervisor tasks

use super::{AgentSupervisor, EnsureProcessResult, SPAWN_BACKOFF};
use crate::agent::process_control::{
    capture_log_cursor, spawn_host_agent_process, wait_for_agent_ready,
};
use crate::agent::supervisor_helpers::{
    kill_and_wait, probe_websocket_auth_blocking, process_state, process_state_mut,
    KillAndWaitOutcome, ProcessKind,
};
use crate::agent::types::AgentWebSocketAuth;
use crate::commands::agent_probe::probe_port_blocking;
use crate::obs::logging;
use std::process::Child;
use std::time::Instant;

impl AgentSupervisor {
    pub(super) async fn ensure_host_running(
        &self,
        bundle: &crate::agent::install::AgentBundlePaths,
        host_port: u16,
        wsl_port: u16,
        auth: &AgentWebSocketAuth,
        force: bool,
    ) -> Result<EnsureProcessResult, String> {
        if self.probe_listening(host_port).await? {
            if self
                .probe_websocket_auth(host_port, &auth.host_ws_token)
                .await?
            {
                return Ok(EnsureProcessResult::AlreadyRunning);
            }
            self.reconcile_recorded_child(ProcessKind::Host, "auth_probe_failed")
                .await?;
            if self.probe_listening(host_port).await? {
                return Err(format!(
                    "Host agent port {host_port} is occupied by a process that rejected the current websocket token"
                ));
            }
        }
        self.reconcile_recorded_child(ProcessKind::Host, "port_probe_failed")
            .await?;
        if !force && self.is_in_backoff(ProcessKind::Host)? {
            return Ok(EnsureProcessResult::Backoff);
        }

        logging::log(
            "info",
            "agent",
            "spawn_start",
            &format!("kind=host port={host_port}"),
        );
        let bundle_for_spawn = bundle.clone();
        let auth = auth.clone();
        let spawned = tauri::async_runtime::spawn_blocking(move || -> Result<Child, String> {
            let log_file = bundle_for_spawn.log_dir_host.join("agent_latest.log");
            let log_offset = capture_log_cursor(&log_file);
            let mut child = spawn_host_agent_process(
                &bundle_for_spawn,
                host_port,
                wsl_port,
                &auth.host_ws_token,
                &auth.wsl_ws_token,
                &auth.host_allowed_origins,
            )?;
            wait_for_agent_ready(
                &mut child,
                host_port,
                ProcessKind::Host.label(),
                &log_file,
                log_offset,
            )?;
            Ok(child)
        })
        .await
        .map_err(|err| format!("Host agent spawn task failed: {err}"))?;

        match spawned {
            Ok(child) => {
                let pid = child.id();
                self.replace_child(ProcessKind::Host, child).await?;
                self.update_last_spawn(ProcessKind::Host)?;
                logging::log(
                    "info",
                    "agent",
                    "spawn_ready",
                    &format!("kind=host port={host_port} pid={pid}"),
                );
                Ok(EnsureProcessResult::Started)
            }
            Err(err) => {
                logging::log(
                    "error",
                    "agent",
                    "spawn_exit_early",
                    &format!("kind=host port={host_port} error={err}"),
                );
                self.set_last_error(Some(err.clone()))?;
                Err(err)
            }
        }
    }

    pub(super) async fn probe_listening(&self, port: u16) -> Result<bool, String> {
        tauri::async_runtime::spawn_blocking(move || probe_port_blocking(port).listening)
            .await
            .map_err(|err| format!("Agent probe task failed: {err}"))
    }

    pub(super) async fn probe_websocket_auth(
        &self,
        port: u16,
        token: &str,
    ) -> Result<bool, String> {
        let token = token.to_string();
        tauri::async_runtime::spawn_blocking(move || probe_websocket_auth_blocking(port, &token))
            .await
            .map_err(|err| format!("Agent websocket auth probe task failed: {err}"))
    }

    pub(super) async fn stop_process(&self, kind: ProcessKind) -> Result<(), String> {
        let mut errors: Vec<String> = Vec::new();

        if matches!(kind, ProcessKind::Wsl) {
            if let Err(err) = self.terminate_wsl_backend_for_reason("stop").await {
                logging::log(
                    "warn",
                    "agent",
                    "stop_cleanup",
                    &format!("kind=wsl phase=in_distro_terminate outcome=failed error={err}"),
                );
                errors.push(format!("WSL in-distro terminate failed during stop: {err}"));
            }
        }

        if let Err(err) = self.reconcile_recorded_child(kind, "stop").await {
            errors.push(err);
        }

        if matches!(kind, ProcessKind::Wsl) {
            if let Err(err) = self.set_wsl_launch_target(None) {
                errors.push(format!("Failed to clear WSL launch target: {err}"));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }

    pub(super) fn is_in_backoff(&self, kind: ProcessKind) -> Result<bool, String> {
        let state = self
            .state
            .lock()
            .map_err(|_| "Agent supervisor lock poisoned".to_string())?;
        match process_state(&state, kind).last_spawn_at {
            Some(last) => Ok(last.elapsed() < SPAWN_BACKOFF),
            None => Ok(false),
        }
    }

    pub(super) fn update_last_spawn(&self, kind: ProcessKind) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Agent supervisor lock poisoned".to_string())?;
        process_state_mut(&mut state, kind).last_spawn_at = Some(Instant::now());
        Ok(())
    }

    pub(super) async fn replace_child(
        &self,
        kind: ProcessKind,
        child: Child,
    ) -> Result<(), String> {
        self.reconcile_recorded_child(kind, "replace_child").await?;
        self.store_child(kind, child)
    }

    pub(super) async fn reconcile_recorded_child(
        &self,
        kind: ProcessKind,
        reason: &str,
    ) -> Result<(), String> {
        let Some(mut child) = self.take_child(kind)? else {
            return Ok(());
        };

        let pid = child.id();
        match child
            .try_wait()
            .map_err(|err| format!("Failed to poll {} process: {err}", kind.label()))?
        {
            Some(status) => {
                logging::log(
                    "info",
                    "agent",
                    "kill_done",
                    &format!(
                        "kind={} pid={pid} reason={reason} outcome=already_exited status={status}",
                        kind.log_key()
                    ),
                );
                Ok(())
            }
            None => {
                logging::log(
                    "info",
                    "agent",
                    "kill_start",
                    &format!("kind={} pid={pid} reason={reason}", kind.log_key()),
                );
                let result = tauri::async_runtime::spawn_blocking(move || kill_and_wait(child))
                    .await
                    .map_err(|err| format!("{} kill task failed: {err}", kind.label()))?;

                match result {
                    KillAndWaitOutcome::Exited(status) => {
                        logging::log(
                            "info",
                            "agent",
                            "kill_done",
                            &format!(
                                "kind={} pid={pid} reason={reason} outcome=killed status={status}",
                                kind.log_key()
                            ),
                        );
                        Ok(())
                    }
                    KillAndWaitOutcome::Failed(child, err) => {
                        self.restore_child(kind, child)?;
                        let message =
                            format!("Failed to terminate {} process: {err}", kind.log_key());
                        logging::log(
                            "error",
                            "agent",
                            "kill_done",
                            &format!(
                                "kind={} pid={pid} reason={reason} outcome=failed error={err}",
                                kind.log_key()
                            ),
                        );
                        Err(message)
                    }
                }
            }
        }
    }

    fn take_child(&self, kind: ProcessKind) -> Result<Option<Child>, String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Agent supervisor lock poisoned".to_string())?;
        Ok(process_state_mut(&mut state, kind).child.take())
    }

    fn restore_child(&self, kind: ProcessKind, mut child: Child) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Agent supervisor lock poisoned".to_string())?;
        let slot = &mut process_state_mut(&mut state, kind).child;
        if slot.is_some() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "Failed to restore {} process handle: slot was already occupied",
                kind.log_key()
            ));
        }
        *slot = Some(child);
        Ok(())
    }

    fn store_child(&self, kind: ProcessKind, mut child: Child) -> Result<(), String> {
        let mut state = self.state.lock().map_err(|_| {
            let _ = child.kill();
            let _ = child.wait();
            "Agent supervisor lock poisoned".to_string()
        })?;
        let slot = &mut process_state_mut(&mut state, kind).child;
        if slot.is_some() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "Failed to store {} process handle: slot already occupied",
                kind.log_key()
            ));
        }
        *slot = Some(child);
        Ok(())
    }

    pub(super) fn set_last_error(&self, message: Option<String>) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Agent supervisor lock poisoned".to_string())?;
        state.last_error = message;
        Ok(())
    }
}
