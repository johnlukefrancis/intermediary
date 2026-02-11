// Path: src-tauri/src/lib/agent/supervisor/processes.rs
// Description: Process lifecycle helpers for host/WSL supervisor tasks

use super::{AgentSupervisor, EnsureProcessResult, SPAWN_BACKOFF};
use crate::agent::process_control::{
    spawn_host_agent_process, spawn_wsl_agent_process, wait_for_agent_ready,
};
use crate::agent::supervisor_helpers::{
    kill_and_wait, process_state, process_state_mut, KillAndWaitOutcome, ProcessKind,
};
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
        force: bool,
    ) -> Result<EnsureProcessResult, String> {
        if self.probe_listening(host_port).await? {
            return Ok(EnsureProcessResult::AlreadyRunning);
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
        let spawned = tauri::async_runtime::spawn_blocking(move || -> Result<Child, String> {
            let mut child = spawn_host_agent_process(&bundle_for_spawn, host_port, wsl_port)?;
            wait_for_agent_ready(
                &mut child,
                host_port,
                ProcessKind::Host.label(),
                false,
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

    pub(super) async fn ensure_wsl_running(
        &self,
        bundle: &crate::agent::install::AgentBundlePaths,
        distro: Option<&str>,
        wsl_port: u16,
        force: bool,
    ) -> Result<EnsureProcessResult, String> {
        if self.probe_listening(wsl_port).await? {
            return Ok(EnsureProcessResult::AlreadyRunning);
        }
        self.reconcile_recorded_child(ProcessKind::Wsl, "port_probe_failed")
            .await?;
        if !force && self.is_in_backoff(ProcessKind::Wsl)? {
            return Ok(EnsureProcessResult::Backoff);
        }

        let distro_value = distro
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("default");
        logging::log(
            "info",
            "agent",
            "spawn_start",
            &format!("kind=wsl port={wsl_port} distro={distro_value}"),
        );
        let bundle_for_spawn = bundle.clone();
        let distro = distro.map(str::to_string);
        let spawned = tauri::async_runtime::spawn_blocking(move || -> Result<Child, String> {
            let mut child =
                spawn_wsl_agent_process(&bundle_for_spawn, distro.as_deref(), wsl_port)?;
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
                    &format!("kind=wsl port={wsl_port} pid={pid}"),
                );
                Ok(EnsureProcessResult::Started)
            }
            Err(err) => {
                logging::log(
                    "error",
                    "agent",
                    "spawn_exit_early",
                    &format!("kind=wsl port={wsl_port} error={err}"),
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

    pub(super) async fn stop_process(&self, kind: ProcessKind) -> Result<(), String> {
        self.reconcile_recorded_child(kind, "stop").await
    }

    fn is_in_backoff(&self, kind: ProcessKind) -> Result<bool, String> {
        let state = self
            .state
            .lock()
            .map_err(|_| "Agent supervisor lock poisoned".to_string())?;
        match process_state(&state, kind).last_spawn_at {
            Some(last) => Ok(last.elapsed() < SPAWN_BACKOFF),
            None => Ok(false),
        }
    }

    fn update_last_spawn(&self, kind: ProcessKind) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Agent supervisor lock poisoned".to_string())?;
        process_state_mut(&mut state, kind).last_spawn_at = Some(Instant::now());
        Ok(())
    }

    async fn replace_child(&self, kind: ProcessKind, child: Child) -> Result<(), String> {
        self.reconcile_recorded_child(kind, "replace_child").await?;
        self.store_child(kind, child)
    }

    async fn reconcile_recorded_child(&self, kind: ProcessKind, reason: &str) -> Result<(), String> {
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
        let mut state = self
            .state
            .lock()
            .map_err(|_| {
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
