// Path: crates/im_host_agent/src/runtime/local_host_backend.rs
// Description: Host-native local backend for repo watch, staging, and bundle operations

use im_agent::bundles::{
    build_bundle, cancel_bundle_build, list_bundles, BuildBundleOptions, BundleCancelToken,
    ListBundlesOptions,
};
use im_agent::error::AgentError;
use im_agent::logging::Logger;
use im_agent::protocol::{
    BuildBundleCommand, BundleBuiltEvent, BundleInfo, ClientHelloCommand, ClientHelloResult,
    ListBundlesCommand, RefreshCommand, RefreshResult, SetOptionsCommand, SetOptionsResult,
    StageFileCommand, StageFileResult, UiResponse, WatchRepoCommand, WatchRepoResult,
};
use im_agent::runtime::{AgentRuntime, RepoConfig, RepoRootKind};
use im_agent::server::EventBus;
use im_agent::staging::{stage_file_for_kind, StageFileCancelToken, StagingRootKind};

use crate::error_codes::REPO_ROOT_MISMATCH;

pub struct LocalHostBackend {
    runtime: AgentRuntime,
}

impl LocalHostBackend {
    pub fn new() -> Self {
        Self {
            runtime: AgentRuntime::new_for_root_kind(RepoRootKind::Host),
        }
    }

    pub async fn apply_client_hello(
        &mut self,
        command: ClientHelloCommand,
        agent_version: &str,
        event_bus: &EventBus,
        logger: &Logger,
    ) -> Result<ClientHelloResult, AgentError> {
        self.runtime
            .apply_client_hello(command, agent_version, event_bus, logger)
            .await
    }

    pub fn apply_set_options(&mut self, command: SetOptionsCommand) -> SetOptionsResult {
        self.runtime.apply_set_options(command)
    }

    pub async fn watch_repo(
        &mut self,
        command: WatchRepoCommand,
        event_bus: &EventBus,
        logger: &Logger,
    ) -> Result<WatchRepoResult, AgentError> {
        self.runtime
            .watch_repo(&command.repo_id, event_bus, logger)
            .await
    }

    pub async fn refresh_repo(&self, command: RefreshCommand) -> Result<RefreshResult, AgentError> {
        self.runtime.refresh_repo(&command.repo_id).await
    }

    pub async fn stage_file(
        &self,
        command: StageFileCommand,
    ) -> Result<StageFileResult, AgentError> {
        let staging = self
            .runtime
            .staging
            .as_ref()
            .cloned()
            .ok_or_else(|| AgentError::new("NOT_CONFIGURED", "Staging not configured"))?;
        let repo_root = self.host_repo_root(&command.repo_id)?;

        let result = stage_file_for_kind(
            &staging,
            &command.repo_id,
            repo_root,
            &command.path,
            StagingRootKind::Host,
            StageFileCancelToken::new(),
        )
        .await?;

        Ok(StageFileResult {
            repo_id: command.repo_id,
            path: command.path,
            host_path: result.host_path.clone(),
            legacy_windows_path: Some(result.host_path),
            wsl_path: result.wsl_path,
            bytes_copied: result.bytes_copied,
            mtime_ms: result.mtime_ms,
        })
    }

    pub async fn build_bundle(
        &self,
        command: BuildBundleCommand,
        event_bus: &EventBus,
        logger: &Logger,
    ) -> Result<UiResponse, AgentError> {
        let staging = self
            .runtime
            .staging
            .as_ref()
            .cloned()
            .ok_or_else(|| AgentError::new("NOT_CONFIGURED", "Staging not configured"))?;
        let repo_config = self.repo_config(&command.repo_id)?;
        let repo_root = self.host_repo_root(&command.repo_id)?;

        let preset = repo_config
            .bundle_presets
            .iter()
            .find(|preset| preset.preset_id == command.preset_id)
            .ok_or_else(|| {
                AgentError::new(
                    "UNKNOWN_PRESET",
                    format!("Unknown preset: {}", command.preset_id),
                )
            })?
            .clone();

        let result = build_bundle(
            BuildBundleOptions {
                repo_id: command.repo_id.clone(),
                repo_root: repo_root.to_string(),
                preset_id: command.preset_id.clone(),
                build_id: command.build_id.clone(),
                preset_name: preset.preset_name,
                selection: command.selection,
                staging,
                staging_kind: StagingRootKind::Host,
                global_excludes: command.global_excludes,
            },
            event_bus,
            logger,
            BundleCancelToken::new(),
        )
        .await?;

        event_bus.broadcast_event(im_agent::protocol::AgentEvent::BundleBuilt(
            BundleBuiltEvent {
                repo_id: command.repo_id.clone(),
                preset_id: command.preset_id.clone(),
                host_path: result.host_path.clone(),
                legacy_windows_path: Some(result.host_path.clone()),
                alias_host_path: result.alias_host_path.clone(),
                legacy_alias_windows_path: Some(result.alias_host_path.clone()),
                bytes: result.bytes,
                file_count: result.file_count,
                built_at_iso: result.built_at_iso.clone(),
            },
        ));

        Ok(UiResponse::BuildBundleResult(
            im_agent::protocol::BuildBundleResult {
                repo_id: command.repo_id,
                preset_id: command.preset_id,
                host_path: result.host_path.clone(),
                legacy_windows_path: Some(result.host_path),
                wsl_path: result.wsl_path,
                alias_host_path: result.alias_host_path.clone(),
                legacy_alias_windows_path: Some(result.alias_host_path),
                bytes: result.bytes,
                file_count: result.file_count,
                built_at_iso: result.built_at_iso,
            },
        ))
    }

    pub fn cancel_bundle_build(
        &self,
        command: im_agent::protocol::CancelBundleBuildCommand,
    ) -> im_agent::protocol::CancelBundleBuildResult {
        let cancelled =
            cancel_bundle_build(&command.repo_id, &command.preset_id, &command.build_id);
        im_agent::protocol::CancelBundleBuildResult {
            repo_id: command.repo_id,
            preset_id: command.preset_id,
            build_id: command.build_id,
            cancelled,
        }
    }

    pub async fn list_bundles(
        &self,
        command: ListBundlesCommand,
    ) -> Result<im_agent::protocol::ListBundlesResult, AgentError> {
        let staging = self
            .runtime
            .staging
            .as_ref()
            .cloned()
            .ok_or_else(|| AgentError::new("NOT_CONFIGURED", "Staging not configured"))?;

        let bundles = list_bundles(ListBundlesOptions {
            staging,
            staging_kind: StagingRootKind::Host,
            repo_id: command.repo_id.clone(),
            preset_id: command.preset_id.clone(),
        })
        .await?;
        let results: Vec<BundleInfo> = bundles;

        Ok(im_agent::protocol::ListBundlesResult {
            repo_id: command.repo_id,
            preset_id: command.preset_id,
            bundles: results,
        })
    }

    pub(crate) fn repo_config(&self, repo_id: &str) -> Result<&RepoConfig, AgentError> {
        self.runtime
            .repo_configs
            .get(repo_id)
            .ok_or_else(|| AgentError::new("UNKNOWN_REPO", format!("Unknown repo: {repo_id}")))
    }

    pub(crate) fn host_repo_root(&self, repo_id: &str) -> Result<&str, AgentError> {
        let repo = self.repo_config(repo_id)?;
        repo.host_root_path().ok_or_else(|| {
            AgentError::new(
                REPO_ROOT_MISMATCH,
                format!("Repo {repo_id} is not routed to host root handling"),
            )
        })
    }
}
