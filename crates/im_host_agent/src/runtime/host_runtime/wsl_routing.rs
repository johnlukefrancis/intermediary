// Path: crates/im_host_agent/src/runtime/host_runtime/wsl_routing.rs
// Description: WSL forwarding, generation-aware clientHello replay, and transport error emission for HostRuntime

use im_agent::error::AgentError;
use im_agent::protocol::{
    AgentErrorDetails, AgentErrorEvent, AgentEvent, ClientHelloCommand, ClientHelloResult,
    UiCommand, UiResponse,
};
use im_agent::server::EventBus;
use tokio::time::timeout;

use crate::error_codes::{WSL_BACKEND_TIMEOUT, WSL_BACKEND_UNAVAILABLE};
use crate::runtime::host_runtime_helpers::repo_id_from_command;
use crate::wsl::WslBackendClient;

use super::{HostRuntime, WSL_CLIENT_HELLO_TIMEOUT};

impl HostRuntime {
    pub(super) async fn replay_cached_wsl_client_hello_if_needed(
        &mut self,
        event_bus: &EventBus,
    ) -> Result<Option<ClientHelloResult>, AgentError> {
        let current_generation = self.wsl_client(event_bus).connection_generation();
        let Some(pending_hello) = self.wsl_client_hello_cache.pending(current_generation) else {
            return Ok(None);
        };

        let result = self
            .forward_client_hello_to_wsl(pending_hello.command.clone(), event_bus)
            .await?;
        self.wsl_client_hello_cache
            .mark_applied_if_latest(&pending_hello.fingerprint, current_generation);
        Ok(Some(result))
    }

    async fn forward_client_hello_to_wsl(
        &mut self,
        command: ClientHelloCommand,
        event_bus: &EventBus,
    ) -> Result<ClientHelloResult, AgentError> {
        let wsl_client = self.wsl_client(event_bus);
        let forward = wsl_client.forward_command(UiCommand::ClientHello(command));

        let forwarded = match timeout(WSL_CLIENT_HELLO_TIMEOUT, forward).await {
            Ok(result) => result,
            Err(_) => {
                return Err(AgentError::new(
                    WSL_BACKEND_TIMEOUT,
                    "Timed out waiting for WSL backend clientHello response",
                ));
            }
        };

        match forwarded {
            Ok(UiResponse::ClientHelloResult(result)) => {
                self.mark_wsl_transport_success(event_bus);
                Ok(result)
            }
            Ok(_) => Err(AgentError::new(
                WSL_BACKEND_UNAVAILABLE,
                "WSL backend returned unexpected response type for clientHello",
            )),
            Err(err) => Err(err),
        }
    }

    pub(super) async fn forward_wsl_command(
        &mut self,
        command: UiCommand,
        event_bus: &EventBus,
    ) -> Result<UiResponse, AgentError> {
        let repo_id = repo_id_from_command(&command).map(str::to_string);

        match self.wsl_client(event_bus).forward_command(command).await {
            Ok(response) => {
                self.mark_wsl_transport_success(event_bus);
                Ok(response)
            }
            Err(err) => {
                self.emit_wsl_unavailable_if_transport_error(&err, event_bus, repo_id);
                Err(err)
            }
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

    pub(super) fn mark_wsl_transport_success(&mut self, event_bus: &EventBus) {
        let generation = self.wsl_client(event_bus).connection_generation();
        self.wsl_transport_state.mark_success(generation);
    }

    pub(super) fn emit_wsl_unavailable_if_transport_error(
        &mut self,
        err: &AgentError,
        event_bus: &EventBus,
        repo_id: Option<String>,
    ) {
        if !is_wsl_transport_error(err) {
            return;
        }

        let generation = self.wsl_client(event_bus).connection_generation();
        if !self
            .wsl_transport_state
            .should_emit_offline_error(generation)
        {
            return;
        }

        self.emit_wsl_backend_error_with_code(
            event_bus,
            err.message().to_string(),
            repo_id,
            err.code(),
        );
    }

    pub(super) fn emit_wsl_backend_error(
        &self,
        err: &AgentError,
        event_bus: &EventBus,
        repo_id: Option<String>,
    ) {
        self.emit_wsl_backend_error_with_code(
            event_bus,
            err.message().to_string(),
            repo_id,
            err.code(),
        );
    }

    fn emit_wsl_backend_error_with_code(
        &self,
        event_bus: &EventBus,
        message: String,
        repo_id: Option<String>,
        raw_code: &str,
    ) {
        let event = AgentErrorEvent::new(
            "wslBackend",
            message.clone(),
            Some(AgentErrorDetails {
                code: None,
                doc_path: None,
                repo_id,
                raw_code: Some(raw_code.to_string()),
                raw_message: Some(message),
            }),
        );

        event_bus.broadcast_event(AgentEvent::Error(event));
    }
}

fn is_wsl_transport_error(err: &AgentError) -> bool {
    err.code() == WSL_BACKEND_UNAVAILABLE || err.code() == WSL_BACKEND_TIMEOUT
}
