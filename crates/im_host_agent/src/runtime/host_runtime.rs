// Path: crates/im_host_agent/src/runtime/host_runtime.rs
// Description: Host runtime that routes protocol commands to host-local or WSL backend

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use im_agent::error::AgentError;
use im_agent::logging::Logger;
use im_agent::protocol::{
    AgentErrorDetails, AgentErrorEvent, AgentEvent, ClientHelloCommand, ClientHelloResult,
    SetOptionsCommand, UiCommand, UiResponse,
};
use im_agent::server::EventBus;
use tokio::time::timeout;

use crate::error_codes::{WSL_BACKEND_TIMEOUT, WSL_BACKEND_UNAVAILABLE};
use crate::runtime::host_runtime_helpers::{
    build_repo_backend_map, build_wsl_client_hello, parse_app_config, repo_id_from_command,
    should_forward_wsl_hello,
};
use crate::runtime::local_host_backend::LocalHostBackend;
use crate::runtime::router::resolve_repo_backend;
use crate::runtime::RepoBackend;
use crate::wsl::WslBackendClient;

const WSL_CLIENT_HELLO_TIMEOUT: Duration = Duration::from_secs(8);

pub struct HostRuntime {
    local_backend: LocalHostBackend,
    repo_backends: HashMap<String, RepoBackend>,
    wsl_client: Option<WslBackendClient>,
    wsl_port: u16,
    logger: Logger,
}

impl HostRuntime {
    pub fn new(wsl_port: u16, logger: Logger) -> Self {
        Self {
            local_backend: LocalHostBackend::new(),
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
        let wsl_repo_ids: Vec<String> = parsed_config
            .repos
            .iter()
            .filter(|repo| repo.root_kind() == im_agent::runtime::RepoRootKind::Wsl)
            .map(|repo| repo.repo_id.clone())
            .collect();

        if cfg!(not(target_os = "windows")) && !wsl_repo_ids.is_empty() {
            return Err(AgentError::new(
                "UNSUPPORTED_REPO_ROOT",
                format!(
                    "WSL repo roots are not supported on this platform (repos: {})",
                    wsl_repo_ids.join(", ")
                ),
            ));
        }

        let had_wsl_repos_before = self.has_wsl_repos();
        self.repo_backends = build_repo_backend_map(&parsed_config);
        let has_wsl_repos_now = self.has_wsl_repos();

        let local_result = self
            .local_backend
            .apply_client_hello(command.clone(), agent_version, event_bus, &self.logger)
            .await?;

        let mut watched_ids: HashSet<String> = local_result.watched_repo_ids.into_iter().collect();

        if cfg!(target_os = "windows")
            && should_forward_wsl_hello(had_wsl_repos_before, has_wsl_repos_now)
        {
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
        if !self.has_wsl_repos() || !cfg!(target_os = "windows") {
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
        let repo_id = repo_id_from_command(&command).map(str::to_string);
        let backend = resolve_repo_backend(&command, &self.repo_backends)?
            .ok_or_else(|| AgentError::new("UNKNOWN_COMMAND", "Unsupported command"))?;

        match backend {
            RepoBackend::Host => self.dispatch_host_command(command, event_bus).await,
            RepoBackend::Wsl if cfg!(target_os = "windows") => {
                self.forward_wsl_command(command, event_bus).await
            }
            RepoBackend::Wsl => Err(Self::unsupported_wsl_root_error(repo_id)),
        }
    }

    async fn dispatch_host_command(
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
        let wsl_client = self.wsl_client(event_bus);
        let forward = wsl_client.forward_command(UiCommand::ClientHello(command));

        let forwarded = match timeout(WSL_CLIENT_HELLO_TIMEOUT, forward).await {
            Ok(result) => result,
            Err(_) => {
                self.emit_wsl_unavailable(
                    event_bus,
                    "Timed out waiting for WSL backend clientHello response".to_string(),
                    None,
                );
                return None;
            }
        };

        match forwarded {
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
        if err.code() != WSL_BACKEND_UNAVAILABLE && err.code() != WSL_BACKEND_TIMEOUT {
            return;
        }
        self.emit_wsl_unavailable_with_code(
            event_bus,
            err.message().to_string(),
            repo_id,
            err.code(),
        );
    }

    fn emit_wsl_unavailable(&self, event_bus: &EventBus, message: String, repo_id: Option<String>) {
        self.emit_wsl_unavailable_with_code(event_bus, message, repo_id, WSL_BACKEND_UNAVAILABLE);
    }

    fn emit_wsl_unavailable_with_code(
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

    fn unsupported_wsl_root_error(repo_id: Option<String>) -> AgentError {
        let repo_suffix = repo_id
            .as_deref()
            .map(|value| format!(" (repo: {value})"))
            .unwrap_or_default();
        AgentError::new(
            "UNSUPPORTED_REPO_ROOT",
            format!("WSL repo roots are not supported on this platform{repo_suffix}"),
        )
    }
}
