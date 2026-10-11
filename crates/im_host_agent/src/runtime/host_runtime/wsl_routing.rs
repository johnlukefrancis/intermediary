// Path: crates/im_host_agent/src/runtime/host_runtime/wsl_routing.rs
// Description: WSL client ownership and lock-free WSL route resolution for HostRuntime

use im_agent::error::AgentError;
use im_agent::protocol::UiCommand;
use im_agent::server::EventBus;

use crate::error_codes::WSL_BACKEND_UNAVAILABLE;
use crate::runtime::router::resolve_repo_backend;
use crate::runtime::RepoBackend;
use crate::wsl::WslBackendClient;

use super::HostRuntime;

impl HostRuntime {
    /// The WSL client for a WSL-routed command, cloned so the caller awaits it after unlocking.
    pub fn wsl_client_for_command(
        &self,
        command: &UiCommand,
    ) -> Result<Option<WslBackendClient>, AgentError> {
        match resolve_repo_backend(command, &self.repo_backends)? {
            Some(RepoBackend::Wsl) if self.wsl_routing_supported => {
                self.wsl_client.clone().map(Some).ok_or_else(|| {
                    AgentError::new(WSL_BACKEND_UNAVAILABLE, "WSL backend client is not started")
                })
            }
            Some(RepoBackend::Wsl) => Err(Self::unsupported_wsl_root_error(
                command.repo_id().map(str::to_string),
            )),
            Some(RepoBackend::Host) | None => Ok(None),
        }
    }

    pub(super) fn wsl_client(&mut self, event_bus: &EventBus) -> WslBackendClient {
        if let Some(client) = &self.wsl_client {
            return client.clone();
        }

        let client = WslBackendClient::new(
            self.wsl_port,
            self.wsl_ws_token.clone(),
            event_bus.clone(),
            self.logger.clone(),
        );
        self.wsl_client = Some(client.clone());
        client
    }

    /// Routes WSL roots through a scripted client, as the Windows build routes them to the real one.
    #[cfg(test)]
    pub(crate) fn with_wsl_client_for_test(
        client: WslBackendClient,
        logger: im_agent::logging::Logger,
    ) -> Self {
        let mut runtime = Self::new(0, "test_token".to_string(), logger);
        runtime.wsl_client = Some(client);
        runtime.wsl_routing_supported = true;
        runtime
    }
}
