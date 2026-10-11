// Path: crates/im_host_agent/src/wsl/wsl_backend_client/routed_forward.rs
// Description: WSL-routed command forwarding that awaits the shared generation hello, plus the hello's public surface

use std::time::Duration;

use im_agent::error::AgentError;
use im_agent::protocol::{ClientHelloCommand, UiCommand, UiResponse};
use tokio::time::{timeout_at, Instant};

use super::hello_state::HelloWait;
use super::timeouts::timeout_for_command;
use super::WslBackendClient;
use crate::error_codes::{WSL_BACKEND_TIMEOUT, WSL_BACKEND_UNAVAILABLE};

impl WslBackendClient {
    /// Records the WSL hello the backend should run; `None` means no WSL config to apply. Never awaits I/O.
    pub fn set_desired_hello(&self, command: Option<ClientHelloCommand>) -> Result<(), AgentError> {
        self.hello.set_desired(command)
    }

    /// Watched ids from the hello applied on the current generation; empty when none applied yet.
    pub fn applied_watched_repo_ids(&self) -> Vec<String> {
        self.hello
            .state()
            .applied_watched_repo_ids(self.connection_generation())
    }

    /// Forwards a repo command once the generation's hello is applied; reports transport status.
    pub async fn forward_routed(&self, command: UiCommand) -> Result<UiResponse, AgentError> {
        let repo_id = command.repo_id().map(str::to_string);
        let budget = timeout_for_command(&command);
        let deadline = Instant::now() + budget;
        self.await_hello(budget).await?;
        let remaining = deadline.saturating_duration_since(Instant::now());
        let result = self.forward_command_with_timeout(command, remaining).await;
        match result {
            Ok(forwarded) => {
                self.status.mark_success(forwarded.generation);
                Ok(forwarded.response)
            }
            Err(err) => {
                self.status.report_transport_error(&err, repo_id);
                Err(err)
            }
        }
    }

    /// Forwards without the hello gate (options carry no repo state); reports transport status.
    pub async fn forward_unrouted(&self, command: UiCommand) -> Result<UiResponse, AgentError> {
        match self.forward_command_with_generation(command).await {
            Ok(forwarded) => {
                self.status.mark_success(forwarded.generation);
                Ok(forwarded.response)
            }
            Err(err) => {
                self.status.report_transport_error(&err, None);
                Err(err)
            }
        }
    }

    /// Stands in for the request loop opening a connection generation.
    #[cfg(test)]
    pub(crate) fn simulate_connected(&self) -> u64 {
        super::hello_reconciler::begin_generation(
            &self.connection_generation,
            &self.hello,
            &self.status,
        )
    }

    /// Waits on the reconciler's shared attempt, never sending a hello itself.
    pub(crate) async fn await_hello(&self, budget: Duration) -> Result<(), AgentError> {
        let mut changes = self.hello.subscribe();
        let seen_attempts = self.hello.state().finished_attempts();
        let deadline = Instant::now() + budget;
        loop {
            let generation = self.connection_generation();
            let status = self.hello.state().wait_status(generation, seen_attempts);
            match status {
                HelloWait::Ready => return Ok(()),
                HelloWait::Failed(err) => return Err(err),
                HelloWait::Kick => self.hello.wake_reconciler(),
                HelloWait::Pending => {}
            }
            match timeout_at(deadline, changes.changed()).await {
                Ok(Ok(())) => {}
                Ok(Err(_)) => {
                    return Err(AgentError::new(
                        WSL_BACKEND_UNAVAILABLE,
                        "WSL clientHello reconciler stopped",
                    ))
                }
                Err(_) => {
                    return Err(AgentError::new(
                        WSL_BACKEND_TIMEOUT,
                        format!(
                            "WSL backend timed out after {}ms waiting for clientHello",
                            budget.as_millis()
                        ),
                    ))
                }
            }
        }
    }
}
