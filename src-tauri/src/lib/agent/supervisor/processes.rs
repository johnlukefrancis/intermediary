// Path: src-tauri/src/lib/agent/supervisor/processes.rs
// Description: Process lifecycle helpers for host/WSL supervisor tasks

use super::{AgentSupervisor, EnsureProcessResult, SPAWN_BACKOFF};
use crate::agent::process_control::{
    spawn_host_agent_process, spawn_wsl_agent_process, wait_for_agent_ready,
};
use crate::agent::supervisor_helpers::{process_state, process_state_mut, ProcessKind};
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
        if !force && self.is_in_backoff(ProcessKind::Host)? {
            return Ok(EnsureProcessResult::Backoff);
        }

        let bundle_for_spawn = bundle.clone();
        let spawned = tauri::async_runtime::spawn_blocking(move || -> Result<Child, String> {
            let mut child = spawn_host_agent_process(&bundle_for_spawn, host_port, wsl_port)?;
            wait_for_agent_ready(&mut child, host_port, ProcessKind::Host.label())?;
            Ok(child)
        })
        .await
        .map_err(|err| format!("Host agent spawn task failed: {err}"))?;

        match spawned {
            Ok(child) => {
                self.store_child(ProcessKind::Host, child)?;
                self.update_last_spawn(ProcessKind::Host)?;
                logging::log(
                    "info",
                    "agent",
                    "start_host",
                    &format!("Spawned host agent on port {host_port}"),
                );
                Ok(EnsureProcessResult::Started)
            }
            Err(err) => {
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
        if !force && self.is_in_backoff(ProcessKind::Wsl)? {
            return Ok(EnsureProcessResult::Backoff);
        }

        let bundle_for_spawn = bundle.clone();
        let distro = distro.map(str::to_string);
        let spawned = tauri::async_runtime::spawn_blocking(move || -> Result<Child, String> {
            let mut child =
                spawn_wsl_agent_process(&bundle_for_spawn, distro.as_deref(), wsl_port)?;
            wait_for_agent_ready(&mut child, wsl_port, ProcessKind::Wsl.label())?;
            Ok(child)
        })
        .await
        .map_err(|err| format!("WSL agent spawn task failed: {err}"))?;

        match spawned {
            Ok(child) => {
                self.store_child(ProcessKind::Wsl, child)?;
                self.update_last_spawn(ProcessKind::Wsl)?;
                logging::log(
                    "info",
                    "agent",
                    "start_wsl",
                    &format!("Spawned WSL agent on port {wsl_port}"),
                );
                Ok(EnsureProcessResult::Started)
            }
            Err(err) => {
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
        let mut child = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| "Agent supervisor lock poisoned".to_string())?;
            process_state_mut(&mut state, kind).child.take()
        };

        if let Some(mut child) = child.take() {
            tauri::async_runtime::spawn_blocking(move || {
                let _ = child.kill();
                let _ = child.wait();
            })
            .await
            .map_err(|err| format!("{} stop task failed: {err}", kind.label()))?;
        }

        Ok(())
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

    fn store_child(&self, kind: ProcessKind, child: Child) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Agent supervisor lock poisoned".to_string())?;
        process_state_mut(&mut state, kind).child = Some(child);
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
