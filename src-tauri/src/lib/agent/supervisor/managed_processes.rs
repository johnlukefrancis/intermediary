// Path: src-tauri/src/lib/agent/supervisor/managed_processes.rs
// Description: Supervisor-owned child-process bookkeeping, stop, and reconciliation helpers

use super::{AgentSupervisor, SPAWN_BACKOFF};
use crate::agent::supervisor::process_kill::{kill_and_wait, KillAndWaitOutcome};
use crate::agent::supervisor::state::{process_state, process_state_mut, ProcessKind};
use crate::obs::logging;
use std::process::Child;
use std::time::Instant;

impl AgentSupervisor {
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

    pub(super) fn set_last_error(&self, message: Option<String>) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Agent supervisor lock poisoned".to_string())?;
        state.last_error = message;
        Ok(())
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
}
