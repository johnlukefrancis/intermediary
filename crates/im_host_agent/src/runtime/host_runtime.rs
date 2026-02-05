// Path: crates/im_host_agent/src/runtime/host_runtime.rs
// Description: Host runtime that routes protocol commands to Windows-local or WSL backend

use std::collections::{HashMap, HashSet};

use im_agent::error::AgentError;
use im_agent::logging::Logger;
use im_agent::protocol::{
    AgentErrorDetails, AgentErrorEvent, AgentEvent, ClientHelloCommand, ClientHelloResult,
    SetOptionsCommand, UiCommand, UiResponse,
};
use im_agent::server::EventBus;

use crate::error_codes::WSL_BACKEND_UNAVAILABLE;
use crate::runtime::host_runtime_helpers::{
    build_repo_backend_map, build_wsl_client_hello, parse_app_config, repo_id_from_command,
};
use crate::runtime::local_windows_backend::LocalWindowsBackend;
use crate::runtime::router::resolve_repo_backend;
use crate::runtime::RepoBackend;
use crate::wsl::WslBackendClient;

pub struct HostRuntime {
    local_backend: LocalWindowsBackend,
    repo_backends: HashMap<String, RepoBackend>,
    wsl_client: Option<WslBackendClient>,
    wsl_port: u16,
    logger: Logger,
}

impl HostRuntime {
    pub fn new(wsl_port: u16, logger: Logger) -> Self {
        Self {
            local_backend: LocalWindowsBackend::new(),
            repo_backends: HashMap::new(),
            wsl_client: None,
            wsl_port,
            logger,
        }
    }

    pub async fn dispatch_command(
        &mut self,
        command: UiCommand,
        agent_version: &str,
        event_bus: &EventBus,
    ) -> Result<UiResponse, AgentError> {
        match command {
            UiCommand::ClientHello(command) => {
                let result = self
                    .handle_client_hello(command, agent_version, event_bus)
                    .await?;
                Ok(UiResponse::ClientHelloResult(result))
            }
            UiCommand::SetOptions(command) => {
                let result = self.handle_set_options(command, event_bus).await;
                Ok(UiResponse::SetOptionsResult(result))
            }
            UiCommand::WatchRepo(_)
            | UiCommand::Refresh(_)
            | UiCommand::StageFile(_)
            | UiCommand::BuildBundle(_)
            | UiCommand::GetRepoTopLevel(_)
            | UiCommand::ListBundles(_) => self.dispatch_repo_command(command, event_bus).await,
            UiCommand::Unknown => Err(AgentError::new("UNKNOWN_COMMAND", "Unsupported command")),
        }
    }

    async fn handle_client_hello(
        &mut self,
        command: ClientHelloCommand,
        agent_version: &str,
        event_bus: &EventBus,
    ) -> Result<ClientHelloResult, AgentError> {
        let parsed_config = parse_app_config(&command.config)?;
        self.repo_backends = build_repo_backend_map(&parsed_config);

        let local_result = self
            .local_backend
            .apply_client_hello(command.clone(), agent_version, event_bus, &self.logger)
            .await?;

        let mut watched_ids: HashSet<String> = local_result.watched_repo_ids.into_iter().collect();

        if self.has_wsl_repos() {
            let wsl_hello = build_wsl_client_hello(&parsed_config, &command)?;
            if let Some(wsl_result) = self.try_forward_client_hello(wsl_hello, event_bus).await {
                watched_ids.extend(wsl_result.watched_repo_ids);
            }
        }

        let mut watched_repo_ids: Vec<String> = watched_ids.into_iter().collect();
        watched_repo_ids.sort();

        Ok(ClientHelloResult {
            agent_version: agent_version.to_string(),
            watched_repo_ids,
        })
    }

    async fn handle_set_options(
        &mut self,
        command: SetOptionsCommand,
        event_bus: &EventBus,
    ) -> im_agent::protocol::SetOptionsResult {
        let result = self.local_backend.apply_set_options(command.clone());
        if !self.has_wsl_repos() {
            return result;
        }

        if let Err(err) = self
            .wsl_client(event_bus)
            .forward_command(UiCommand::SetOptions(command))
            .await
        {
            self.emit_wsl_unavailable_if_transport_error(&err, event_bus, None);
        }

        result
    }

    async fn dispatch_repo_command(
        &mut self,
        command: UiCommand,
        event_bus: &EventBus,
    ) -> Result<UiResponse, AgentError> {
        let backend = resolve_repo_backend(&command, &self.repo_backends)?
            .ok_or_else(|| AgentError::new("UNKNOWN_COMMAND", "Unsupported command"))?;

        match backend {
            RepoBackend::Windows => self.dispatch_windows_command(command, event_bus).await,
            RepoBackend::Wsl => self.forward_wsl_command(command, event_bus).await,
        }
    }

    async fn dispatch_windows_command(
        &mut self,
        command: UiCommand,
        event_bus: &EventBus,
    ) -> Result<UiResponse, AgentError> {
        match command {
            UiCommand::WatchRepo(command) => {
                let result = self
                    .local_backend
                    .watch_repo(command, event_bus, &self.logger)
                    .await?;
                Ok(UiResponse::WatchRepoResult(result))
            }
            UiCommand::Refresh(command) => {
                let result = self.local_backend.refresh_repo(command).await?;
                Ok(UiResponse::RefreshResult(result))
            }
            UiCommand::StageFile(command) => {
                let result = self.local_backend.stage_file(command).await?;
                Ok(UiResponse::StageFileResult(result))
            }
            UiCommand::BuildBundle(command) => {
                self.local_backend
                    .build_bundle(command, event_bus, &self.logger)
                    .await
            }
            UiCommand::GetRepoTopLevel(command) => {
                let result = self.local_backend.get_repo_top_level(command).await?;
                Ok(UiResponse::GetRepoTopLevelResult(result))
            }
            UiCommand::ListBundles(command) => {
                let result = self.local_backend.list_bundles(command).await?;
                Ok(UiResponse::ListBundlesResult(result))
            }
            UiCommand::ClientHello(_) | UiCommand::SetOptions(_) | UiCommand::Unknown => {
                Err(AgentError::new("UNKNOWN_COMMAND", "Unsupported command"))
            }
        }
    }

    async fn try_forward_client_hello(
        &mut self,
        command: ClientHelloCommand,
        event_bus: &EventBus,
    ) -> Option<ClientHelloResult> {
        match self
            .wsl_client(event_bus)
            .forward_command(UiCommand::ClientHello(command))
            .await
        {
            Ok(UiResponse::ClientHelloResult(result)) => Some(result),
            Ok(_) => {
                self.emit_wsl_unavailable(
                    event_bus,
                    "WSL backend returned unexpected response type for clientHello".to_string(),
                    None,
                );
                None
            }
            Err(err) => {
                self.emit_wsl_unavailable(event_bus, err.message().to_string(), None);
                None
            }
        }
    }

    async fn forward_wsl_command(
        &mut self,
        command: UiCommand,
        event_bus: &EventBus,
    ) -> Result<UiResponse, AgentError> {
        let repo_id = repo_id_from_command(&command).map(str::to_string);

        match self.wsl_client(event_bus).forward_command(command).await {
            Ok(response) => Ok(response),
            Err(err) => {
                self.emit_wsl_unavailable_if_transport_error(&err, event_bus, repo_id);
                Err(err)
            }
        }
    }

    fn wsl_client(&mut self, event_bus: &EventBus) -> WslBackendClient {
        if let Some(client) = &self.wsl_client {
            return client.clone();
        }

        let client = WslBackendClient::new(self.wsl_port, event_bus.clone(), self.logger.clone());
        self.wsl_client = Some(client.clone());
        client
    }

    fn has_wsl_repos(&self) -> bool {
        self.repo_backends
            .values()
            .any(|backend| *backend == RepoBackend::Wsl)
    }

    fn emit_wsl_unavailable_if_transport_error(
        &self,
        err: &AgentError,
        event_bus: &EventBus,
        repo_id: Option<String>,
    ) {
        if err.code() != WSL_BACKEND_UNAVAILABLE {
            return;
        }
        self.emit_wsl_unavailable(event_bus, err.message().to_string(), repo_id);
    }

    fn emit_wsl_unavailable(&self, event_bus: &EventBus, message: String, repo_id: Option<String>) {
        let event = AgentErrorEvent::new(
            "wslBackend",
            message.clone(),
            Some(AgentErrorDetails {
                code: None,
                doc_path: None,
                repo_id,
                raw_code: Some(WSL_BACKEND_UNAVAILABLE.to_string()),
                raw_message: Some(message),
            }),
        );

        event_bus.broadcast_event(AgentEvent::Error(event));
    }
}
