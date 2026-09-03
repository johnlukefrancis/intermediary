// Path: src-tauri/src/lib/agent/supervisor/graceful_stop.rs
// Description: Ask the managed host agent to drain and exit before any kill path runs

use super::shutdown_ws_client::request_agent_shutdown_blocking;
use super::state::{process_state_mut, HostBackendHandle, ProcessKind};
use super::AgentSupervisor;
use crate::obs::logging;
use std::thread;
use std::time::{Duration, Instant};

/// The whole graceful stop — request, wait, and process exit — is bounded by
/// this. Sized above the host agent's own emergency drain bound (450 s,
/// `im_agent::server::SHUTDOWN_EMERGENCY_BOUND`, shared with the WSL backend
/// it forwards to) plus margin for the request/response hop, so a genuine
/// `drained: true` or `drained: false` answer always arrives before this
/// expires; when it does expire anyway (no ack at all, or the process outlives
/// its own honest answer) the caller falls through to `Child::kill`, the
/// emergency bound, not the plan.
const HOST_STOP_WAIT_BOUND: Duration = Duration::from_secs(480);
const EXIT_POLL_INTERVAL: Duration = Duration::from_millis(250);

/// Which route actually stopped the host agent this time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GracefulStopPath {
    /// No host agent was ever recorded this session; nothing to ask.
    NotAttempted,
    /// An explicit `drained: true` ack arrived. The only label finality this
    /// module can vouch for as clean.
    Drained,
    /// The process is gone (or was never reachable), but no `drained: true`
    /// ack ever proved it: `drained: false`, a refusal, or no answer at all.
    /// Never treated as `Drained` — the process may have crashed rather than
    /// finished.
    Unknown,
    /// Neither an ack nor the process disappearing arrived inside the budget.
    /// The kill path is next.
    Incomplete,
}

impl AgentSupervisor {
    /// Requests a drain from the host agent and waits for it to reach a
    /// terminal state. Never fails the caller: a graceful stop that cannot
    /// happen is a logged fact, and the kill path behind it is what
    /// guarantees the process is gone.
    ///
    /// Only an explicit `drained: true` ack earns the `Drained` label.
    /// `drained: false`, a refused request, or no answer at all still waits
    /// for the process up to the full budget before falling to the kill path
    /// — killing on the first sign of trouble is exactly the P0 this guards
    /// against — but a process that disappears without that ack is logged
    /// `Unknown`, never `Drained`: it may have crashed rather than finished.
    pub(super) async fn stop_host_gracefully(&self, reason: &str) -> GracefulStopPath {
        self.stop_host_gracefully_bounded(reason, HOST_STOP_WAIT_BOUND)
            .await
    }

    /// The same route, with the wait bound as a parameter rather than the
    /// production constant: a real 480 s wait has no place in a test, so the
    /// decision logic itself is what a short bound here exercises.
    pub(super) async fn stop_host_gracefully_bounded(
        &self,
        reason: &str,
        wait_bound: Duration,
    ) -> GracefulStopPath {
        let Some(handle) = self.last_host_backend_snapshot() else {
            logging::log(
                "info",
                "agent",
                "graceful_stop",
                &format!("kind=host reason={reason} outcome=not_attempted cause=no_recorded_agent"),
            );
            return GracefulStopPath::NotAttempted;
        };

        let deadline = Instant::now() + wait_bound;
        let port = handle.port;
        let token = handle.ws_token.clone();
        logging::log(
            "info",
            "agent",
            "graceful_stop_start",
            &format!(
                "kind=host reason={reason} port={port} budgetMs={}",
                wait_bound.as_millis()
            ),
        );

        let request_budget = wait_bound;
        let ack = tauri::async_runtime::spawn_blocking(move || {
            request_agent_shutdown_blocking(port, &token, request_budget)
        })
        .await;

        let ack_drained = match ack {
            Ok(Ok(ack)) => {
                logging::log(
                    "info",
                    "agent",
                    "graceful_stop_ack",
                    &format!(
                        "kind=host reason={reason} port={port} drained={} activeMutations={}",
                        ack.drained, ack.active_mutations
                    ),
                );
                Some(ack.drained)
            }
            Ok(Err(err)) => {
                logging::log(
                    "warn",
                    "agent",
                    "graceful_stop_ack",
                    &format!("kind=host reason={reason} port={port} outcome=failed error={err}"),
                );
                None
            }
            Err(err) => {
                logging::log(
                    "error",
                    "agent",
                    "graceful_stop_ack",
                    &format!(
                        "kind=host reason={reason} port={port} outcome=task_failed error={err}"
                    ),
                );
                None
            }
        };

        if ack_drained == Some(true) {
            // Still wait for the process itself, bounded by whatever remains:
            // the agent schedules its own exit right after answering, but the
            // `Drained` label is already earned by the ack, not by this wait.
            let _ = self.await_host_exit(deadline).await;
            logging::log(
                "info",
                "agent",
                "graceful_stop_done",
                &format!("kind=host reason={reason} port={port} outcome=drained"),
            );
            return GracefulStopPath::Drained;
        }

        let exited = self.await_host_exit(deadline).await;
        let outcome = if exited { "unknown" } else { "incomplete" };
        logging::log(
            "warn",
            "agent",
            "graceful_stop_done",
            &format!("kind=host reason={reason} port={port} outcome={outcome}"),
        );
        if exited {
            GracefulStopPath::Unknown
        } else {
            GracefulStopPath::Incomplete
        }
    }

    /// Waits for the agent to be gone: the recorded child's exit when we own
    /// one, otherwise the port going quiet for an agent we adopted.
    async fn await_host_exit(&self, deadline: Instant) -> bool {
        loop {
            match self.recorded_host_child_exited() {
                Some(true) => return true,
                Some(false) => {}
                None => {
                    let port = match self.last_host_backend_snapshot() {
                        Some(handle) => handle.port,
                        None => return true,
                    };
                    if !self.probe_listening(port).await.unwrap_or(false) {
                        return true;
                    }
                }
            }
            if Instant::now() >= deadline {
                return false;
            }
            // The app carries no async-timer dependency of its own; the wait
            // between polls therefore happens on the blocking pool, never on the
            // thread that runs the UI.
            let _ =
                tauri::async_runtime::spawn_blocking(|| thread::sleep(EXIT_POLL_INTERVAL)).await;
        }
    }

    /// `None` when no child handle is recorded (an adopted agent). The state
    /// lock is held only for the poll itself, never across an await.
    fn recorded_host_child_exited(&self) -> Option<bool> {
        let mut state = self.state.lock().ok()?;
        let child = process_state_mut(&mut state, ProcessKind::Host)
            .child
            .as_mut()?;
        match child.try_wait() {
            Ok(status) => Some(status.is_some()),
            Err(_) => Some(false),
        }
    }

    pub(super) fn record_owned_host_backend(
        &self,
        port: u16,
        ws_token: &str,
    ) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Agent supervisor lock poisoned".to_string())?;
        state.last_host_backend = Some(HostBackendHandle {
            port,
            ws_token: ws_token.to_string(),
        });
        Ok(())
    }

    fn last_host_backend_snapshot(&self) -> Option<HostBackendHandle> {
        self.state.lock().ok()?.last_host_backend.clone()
    }

    /// Records how the host agent's stop actually ended, so app-exit teardown
    /// can gate WSL distro termination on it without re-running the stop.
    pub(super) fn record_host_stop_finality(&self, path: GracefulStopPath) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Agent supervisor lock poisoned".to_string())?;
        state.last_host_stop_finality = Some(path);
        Ok(())
    }

    pub(super) fn last_host_stop_finality_snapshot(&self) -> Option<GracefulStopPath> {
        self.state.lock().ok()?.last_host_stop_finality
    }
}

#[cfg(test)]
mod tests;
