// Path: crates/im_host_agent/src/runtime/host_runtime/wsl_routing.rs
// Description: WSL forwarding, generation-aware clientHello replay, and transport error emission for HostRuntime

use std::time::Instant;

use im_agent::error::AgentError;
use im_agent::protocol::{
    AgentErrorDetails, AgentErrorEvent, AgentEvent, ClientHelloCommand, ClientHelloResult,
    UiCommand, UiResponse, WslBackendConnectionStatus, WslBackendStatusEvent,
};
use im_agent::server::EventBus;

use crate::error_codes::{WSL_BACKEND_TIMEOUT, WSL_BACKEND_UNAVAILABLE};
use crate::runtime::host_runtime_helpers::repo_id_from_command;
use crate::wsl::WslBackendClient;

use super::HostRuntime;

impl HostRuntime {
    pub(super) async fn replay_cached_wsl_client_hello_if_needed(
        &mut self,
        event_bus: &EventBus,
    ) -> Result<Option<ClientHelloResult>, AgentError> {
        let current_generation = self.wsl_client(event_bus).connection_generation();
        let Some(pending_hello) = self.wsl_client_hello_cache.pending(current_generation) else {
            return Ok(None);
        };

        let (result, applied_generation) = self
            .forward_client_hello_to_wsl(pending_hello.command.clone(), event_bus)
            .await?;
        self.wsl_client_hello_cache
            .mark_applied_if_latest(&pending_hello.fingerprint, applied_generation);
        Ok(Some(result))
    }

    async fn forward_client_hello_to_wsl(
        &mut self,
        command: ClientHelloCommand,
        event_bus: &EventBus,
    ) -> Result<(ClientHelloResult, u64), AgentError> {
        let wsl_client = self.wsl_client(event_bus);
        let started = Instant::now();
        let forwarded = wsl_client
            .forward_command_with_generation(UiCommand::ClientHello(command))
            .await;

        match forwarded {
            Ok(forwarded) => match forwarded.response {
                UiResponse::ClientHelloResult(result) => {
                    self.mark_wsl_transport_success_for_generation(event_bus, forwarded.generation);
                    self.logger.info(
                        "WSL backend clientHello applied",
                        Some(serde_json::json!({
                            "durationMs": started.elapsed().as_millis(),
                            "generation": forwarded.generation,
                            "watchedRepoCount": result.watched_repo_ids.len()
                        })),
                    );
                    Ok((result, forwarded.generation))
                }
                _ => Err(AgentError::new(
                    WSL_BACKEND_UNAVAILABLE,
                    "WSL backend returned unexpected response type for clientHello",
                )),
            },
            Err(err) => {
                self.logger.warn(
                    "WSL backend clientHello failed",
                    Some(serde_json::json!({
                        "code": err.code(),
                        "durationMs": started.elapsed().as_millis(),
                        "generation": wsl_client.connection_generation()
                    })),
                );
                Err(err)
            }
        }
    }

    pub(super) async fn forward_wsl_command(
        &mut self,
        command: UiCommand,
        event_bus: &EventBus,
    ) -> Result<UiResponse, AgentError> {
        let repo_id = repo_id_from_command(&command).map(str::to_string);

        match self
            .wsl_client(event_bus)
            .forward_command_with_generation(command)
            .await
        {
            Ok(forwarded) => {
                self.mark_wsl_transport_success_for_generation(event_bus, forwarded.generation);
                Ok(forwarded.response)
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

    pub(super) fn mark_wsl_transport_success_for_generation(
        &mut self,
        event_bus: &EventBus,
        generation: u64,
    ) {
        if !self.wsl_transport_state.mark_success(generation) {
            return;
        }

        event_bus.broadcast_event(AgentEvent::WslBackendStatus(WslBackendStatusEvent {
            status: WslBackendConnectionStatus::Online,
            generation,
        }));
    }

    pub(super) fn emit_wsl_unavailable_if_transport_error(
        &mut self,
        err: &AgentError,
        event_bus: &EventBus,
        repo_id: Option<String>,
    ) {
        let generation = self.wsl_client(event_bus).connection_generation();
        self.emit_wsl_unavailable_if_transport_error_for_generation(
            err, event_bus, repo_id, generation,
        );
    }

    fn emit_wsl_unavailable_if_transport_error_for_generation(
        &mut self,
        err: &AgentError,
        event_bus: &EventBus,
        repo_id: Option<String>,
        generation: u64,
    ) {
        if !is_wsl_transport_error(err) {
            return;
        }

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

#[cfg(test)]
#[path = "wsl_routing_tests.rs"]
mod tests;
