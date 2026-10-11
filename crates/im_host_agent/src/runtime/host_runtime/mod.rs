// Path: crates/im_host_agent/src/runtime/host_runtime/mod.rs
// Description: Host runtime command routing and clientHello orchestration for host and WSL backends

mod bundle_forwarding;
mod host_dispatch;
mod shutdown_targets;
mod wsl_routing;

use std::collections::{HashMap, HashSet};

use im_agent::error::AgentError;
use im_agent::logging::Logger;
use im_agent::protocol::{
    ClientHelloCommand, ClientHelloResult, SetOptionsCommand, SetOptionsResult, UiCommand,
    UiResponse,
};
use im_agent::server::EventBus;

use crate::runtime::host_runtime_helpers::{
    build_repo_backend_map, build_wsl_client_hello, parse_app_config, should_forward_wsl_hello,
};
use crate::runtime::local_host_backend::LocalHostBackend;
use crate::runtime::local_host_source_control_backend::HostSourceControlContext;
use crate::runtime::router::resolve_repo_backend;
use crate::runtime::tr_fleet_service::TrFleetService;
use crate::runtime::RepoBackend;
use crate::wsl::WslBackendClient;

pub use self::shutdown_targets::HostShutdownTargets;

pub struct HostRuntime {
    local_backend: LocalHostBackend,
    repo_backends: HashMap<String, RepoBackend>,
    wsl_client: Option<WslBackendClient>,
    /// WSL roots route only on Windows; tests enable it with a scripted client.
    wsl_routing_supported: bool,
    wsl_port: u16,
    wsl_ws_token: String,
    tr_fleet_service: TrFleetService,
    logger: Logger,
}

impl HostRuntime {
    pub fn new(wsl_port: u16, wsl_ws_token: String, logger: Logger) -> Self {
        Self {
            local_backend: LocalHostBackend::new(),
            repo_backends: HashMap::new(),
            wsl_client: None,
            wsl_routing_supported: cfg!(target_os = "windows"),
            wsl_port,
            wsl_ws_token,
            tr_fleet_service: TrFleetService::new(),
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
            UiCommand::GetTrFleetStatus(_) => {
                let result = self.tr_fleet_service.get_tr_fleet_status().await;
                Ok(UiResponse::GetTrFleetStatusResult(result))
            }
            UiCommand::TrFleetAction(command) => {
                let result = self.tr_fleet_service.run_tr_fleet_action(command).await?;
                Ok(UiResponse::TrFleetActionResult(result))
            }
            UiCommand::WatchRepo(_)
            | UiCommand::Refresh(_)
            | UiCommand::StageFile(_)
            | UiCommand::ReadTextFile(_)
            | UiCommand::ReadImageFile(_)
            | UiCommand::InspectBundleEntry(_)
            | UiCommand::BuildBundle(_)
            | UiCommand::CancelBundleBuild(_)
            | UiCommand::GetRepoTopLevel(_)
            | UiCommand::ListRepoDirectory(_)
            | UiCommand::ListBundles(_)
            | UiCommand::SourceControlStatus(_)
            | UiCommand::SourceControlDiff(_)
            | UiCommand::SourceControlImageDiff(_)
            | UiCommand::SourceControlAction(_)
            | UiCommand::ImportFiles(_)
            | UiCommand::WorktreeAction(_) => self.dispatch_repo_command(command, event_bus).await,
            // The server owns these routes so their WSL waits and drains hold no runtime lock.
            UiCommand::Shutdown | UiCommand::SetOptions(_) => Err(AgentError::internal(
                "Shutdown and setOptions must be dispatched without the runtime lock",
            )),
            UiCommand::Unknown => Err(AgentError::new("UNKNOWN_COMMAND", "Unsupported command")),
        }
    }

    /// Cloned host context for a source-control command so Git can run after
    /// the runtime lock is released (see server dispatch).
    pub fn host_source_control_context(
        &self,
        repo_id: &str,
    ) -> Result<HostSourceControlContext, AgentError> {
        self.local_backend.source_control_context(repo_id)
    }

    /// Applies host config and records the WSL hello for the client's reconciler; never awaits WSL I/O.
    async fn handle_client_hello(
        &mut self,
        command: ClientHelloCommand,
        agent_version: &str,
        event_bus: &EventBus,
    ) -> Result<ClientHelloResult, AgentError> {
        let parsed_config = parse_app_config(&command.config)?;
        let wsl_repo_ids: HashSet<String> = parsed_config
            .repos
            .iter()
            .filter(|repo| repo.root_kind() == im_agent::runtime::RepoRootKind::Wsl)
            .map(|repo| repo.repo_id.clone())
            .collect();

        if !self.wsl_routing_supported && !wsl_repo_ids.is_empty() {
            let mut sorted: Vec<&str> = wsl_repo_ids.iter().map(String::as_str).collect();
            sorted.sort_unstable();
            return Err(AgentError::new(
                "UNSUPPORTED_REPO_ROOT",
                format!(
                    "WSL repo roots are not supported on this platform (repos: {})",
                    sorted.join(", ")
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

        if self.wsl_routing_supported
            && should_forward_wsl_hello(had_wsl_repos_before, has_wsl_repos_now)
        {
            let wsl_hello = build_wsl_client_hello(&parsed_config, &command)?;
            let client = self.wsl_client(event_bus);
            client.set_desired_hello(Some(wsl_hello))?;
            watched_ids.extend(
                client
                    .applied_watched_repo_ids()
                    .into_iter()
                    .filter(|repo_id| wsl_repo_ids.contains(repo_id)),
            );
        } else if let Some(client) = &self.wsl_client {
            client.set_desired_hello(None)?;
        }

        let mut watched_repo_ids: Vec<String> = watched_ids.into_iter().collect();
        watched_repo_ids.sort();

        Ok(ClientHelloResult {
            agent_version: agent_version.to_string(),
            watched_repo_ids,
        })
    }

    /// Applies options locally and returns the WSL client the caller forwards them to after unlocking.
    pub fn apply_set_options(
        &mut self,
        command: SetOptionsCommand,
    ) -> (SetOptionsResult, Option<WslBackendClient>) {
        let result = self.local_backend.apply_set_options(command);
        let client = if self.wsl_routing_supported && self.has_wsl_repos() {
            self.wsl_client.clone()
        } else {
            None
        };
        (result, client)
    }

    async fn dispatch_repo_command(
        &mut self,
        command: UiCommand,
        event_bus: &EventBus,
    ) -> Result<UiResponse, AgentError> {
        let repo_id = command.repo_id().map(str::to_string);
        let backend = resolve_repo_backend(&command, &self.repo_backends)?
            .ok_or_else(|| AgentError::new("UNKNOWN_COMMAND", "Unsupported command"))?;

        match backend {
            RepoBackend::Host => self.dispatch_host_command(command, event_bus).await,
            RepoBackend::Wsl if self.wsl_routing_supported => Err(AgentError::internal(
                "WSL-routed commands must be forwarded without the runtime lock",
            )),
            RepoBackend::Wsl => Err(Self::unsupported_wsl_root_error(repo_id)),
        }
    }

    fn has_wsl_repos(&self) -> bool {
        self.repo_backends
            .values()
            .any(|backend| *backend == RepoBackend::Wsl)
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
