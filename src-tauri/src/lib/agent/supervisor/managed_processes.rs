// Path: src-tauri/src/lib/agent/supervisor/managed_processes.rs
// Description: Supervisor-owned child-process bookkeeping, stop, and reconciliation helpers

use super::graceful_stop::GracefulStopPath;
use super::{AgentSupervisor, SPAWN_BACKOFF};
use crate::agent::supervisor::process_kill::{
    discard_process, kill_and_wait, terminate_tree, KillAndWaitOutcome,
};
use crate::agent::supervisor::state::{
    process_state, process_state_mut, ProcessKind, SupervisedChild,
};
use crate::obs::logging;
use std::time::Instant;

impl AgentSupervisor {
    pub(super) async fn stop_process(&self, kind: ProcessKind) -> Result<(), String> {
        let mut errors: Vec<String> = Vec::new();

        // The host agent is asked to drain before anything kills it: a killed
        // `git commit` leaves `.git/index.lock` behind, and the host agent is
        // also the only route that can drain the WSL agent behind it. The kill
        // path below stays exactly as it was — it is the emergency bound — and
        // the reason it records names the route that actually ran.
        let mut reason = "stop";
        if matches!(kind, ProcessKind::Host) {
            let path = self.stop_host_gracefully("stop").await;
            if let Err(err) = self.record_host_stop_finality(path) {
                logging::log(
                    "warn",
                    "agent",
                    "stop_cleanup",
                    &format!("kind=host phase=record_finality outcome=failed error={err}"),
                );
            }
            reason = match path {
                GracefulStopPath::Drained => "stop_after_drain",
                GracefulStopPath::Unknown => "stop_after_unknown_finality",
                GracefulStopPath::Incomplete => "stop_after_incomplete_drain",
                GracefulStopPath::NotAttempted => "stop",
            };
        }

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

        if let Err(err) = self.reconcile_recorded_child(kind, reason).await {
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

    /// Records a freshly started process, after the one it replaces has been
    /// reconciled: that reconciliation is what terminates and drops the stale
    /// tree owner, so two owners are never recorded and none is ever dropped
    /// while the processes inside it are still running.
    pub(super) async fn replace_child(
        &self,
        kind: ProcessKind,
        process: impl Into<SupervisedChild>,
    ) -> Result<(), String> {
        self.reconcile_recorded_child(kind, "replace_child").await?;
        self.store_child(kind, process.into())
    }

    pub(super) async fn reconcile_recorded_child(
        &self,
        kind: ProcessKind,
        reason: &str,
    ) -> Result<(), String> {
        let Some(mut process) = self.take_child(kind)? else {
            return Ok(());
        };

        let pid = process.child.id();
        match process
            .child
            .try_wait()
            .map_err(|err| format!("Failed to poll {} process: {err}", kind.label()))?
        {
            Some(status) => {
                // The process is gone but whatever it started is not: the tree
                // owner outlives it and kills nothing when dropped, so it is
                // spent here rather than released.
                terminate_tree(process.job.as_ref(), pid);
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
                let result = tauri::async_runtime::spawn_blocking(move || kill_and_wait(process))
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
                    KillAndWaitOutcome::Failed(process, err) => {
                        self.restore_child(kind, process)?;
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

    /// The child and its tree owner leave the slot together: a stop that held
    /// only one of them could not end the other.
    fn take_child(&self, kind: ProcessKind) -> Result<Option<SupervisedChild>, String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Agent supervisor lock poisoned".to_string())?;
        Ok(process_state_mut(&mut state, kind).process.take())
    }

    fn restore_child(&self, kind: ProcessKind, process: SupervisedChild) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Agent supervisor lock poisoned".to_string())?;
        let slot = &mut process_state_mut(&mut state, kind).process;
        if slot.is_some() {
            discard_process(process);
            return Err(format!(
                "Failed to restore {} process handle: slot was already occupied",
                kind.log_key()
            ));
        }
        *slot = Some(process);
        Ok(())
    }

    fn store_child(&self, kind: ProcessKind, process: SupervisedChild) -> Result<(), String> {
        let mut state = match self.state.lock() {
            Ok(state) => state,
            Err(_) => {
                discard_process(process);
                return Err("Agent supervisor lock poisoned".to_string());
            }
        };
        let slot = &mut process_state_mut(&mut state, kind).process;
        if slot.is_some() {
            discard_process(process);
            return Err(format!(
                "Failed to store {} process handle: slot already occupied",
                kind.log_key()
            ));
        }
        *slot = Some(process);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
