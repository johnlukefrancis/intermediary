// Path: crates/im_host_agent/src/runtime/host_runtime/bundle_forwarding.rs
// Description: Build-bundle host dispatch and WSL forwarding helpers for HostRuntime

use im_agent::error::AgentError;
use im_agent::protocol::{BuildBundleCommand, CancelBundleBuildCommand, UiCommand, UiResponse};
use im_agent::server::EventBus;

use crate::runtime::router::resolve_repo_backend;
use crate::runtime::RepoBackend;
use crate::wsl::WslBackendClient;

use super::HostRuntime;

impl HostRuntime {
    pub async fn dispatch_host_build_bundle(
        &self,
        command: BuildBundleCommand,
        event_bus: &EventBus,
    ) -> Result<UiResponse, AgentError> {
        self.local_backend
            .build_bundle(command, event_bus, &self.logger)
            .await
    }

    pub fn dispatch_host_cancel_bundle_build(
        &self,
        command: CancelBundleBuildCommand,
    ) -> UiResponse {
        UiResponse::CancelBundleBuildResult(self.local_backend.cancel_bundle_build(command))
    }

    pub fn repo_backend_for_command(
        &self,
        command: &UiCommand,
    ) -> Result<Option<RepoBackend>, AgentError> {
        resolve_repo_backend(command, &self.repo_backends)
    }

    pub async fn prepare_wsl_client_for_command(
        &mut self,
        command: &UiCommand,
        event_bus: &EventBus,
    ) -> Result<WslBackendClient, AgentError> {
        let backend = resolve_repo_backend(command, &self.repo_backends)?
            .ok_or_else(|| AgentError::new("UNKNOWN_COMMAND", "Unsupported command"))?;
        match backend {
            RepoBackend::Host => Err(AgentError::new(
                "REPO_ROOT_MISMATCH",
                "Command is routed to the host backend",
            )),
            RepoBackend::Wsl if cfg!(target_os = "windows") => {
                self.replay_cached_wsl_client_hello_if_needed(event_bus)
                    .await?;
                Ok(self.wsl_client(event_bus))
            }
            RepoBackend::Wsl => Err(Self::unsupported_wsl_root_error(
                command.repo_id().map(str::to_string),
            )),
        }
    }

    pub fn mark_wsl_forward_success(&mut self, event_bus: &EventBus, generation: u64) {
        self.mark_wsl_transport_success_for_generation(event_bus, generation);
    }

    pub fn emit_wsl_forward_error(
        &mut self,
        err: &AgentError,
        event_bus: &EventBus,
        repo_id: Option<String>,
    ) {
        self.emit_wsl_unavailable_if_transport_error(err, event_bus, repo_id);
    }
}
