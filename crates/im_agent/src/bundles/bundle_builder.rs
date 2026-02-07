// Path: crates/im_agent/src/bundles/bundle_builder.rs
// Description: Bundle build orchestration using the im_bundle library

use chrono::Utc;

use crate::error::AgentError;
use crate::logging::Logger;
use crate::protocol::{AgentEvent, BundleBuildProgressEvent, BundleSelection, GlobalExcludes};
use crate::server::EventBus;
use crate::staging::{PathBridgeConfig, StagingRootKind};

use super::bundle_builder_blocking::{
    build_bundle_blocking, format_timestamp, BuildBundleBlockingOptions,
};
use super::bundle_progress::start_progress_forwarder;
use super::git_info::get_git_info;

pub struct BuildBundleOptions {
    pub repo_id: String,
    pub repo_root: String,
    pub preset_id: String,
    pub preset_name: String,
    pub selection: BundleSelection,
    pub staging: PathBridgeConfig,
    pub staging_kind: StagingRootKind,
    pub global_excludes: Option<GlobalExcludes>,
}

pub struct BuildBundleResult {
    pub host_path: String,
    pub wsl_path: Option<String>,
    pub alias_host_path: String,
    pub bytes: u64,
    pub file_count: u64,
    pub built_at_iso: String,
}

impl From<BuildBundleOptions> for BuildBundleBlockingOptions {
    fn from(options: BuildBundleOptions) -> Self {
        Self {
            repo_id: options.repo_id,
            repo_root: options.repo_root,
            preset_id: options.preset_id,
            preset_name: options.preset_name,
            selection: options.selection,
            staging: options.staging,
            staging_kind: options.staging_kind,
            global_excludes: options.global_excludes,
        }
    }
}

pub async fn build_bundle(
    options: BuildBundleOptions,
    event_bus: &EventBus,
    logger: &Logger,
) -> Result<BuildBundleResult, AgentError> {
    let built_at = Utc::now();
    let built_at_iso = built_at.to_rfc3339();
    let timestamp = format_timestamp(built_at);

    let git_info = get_git_info(&options.repo_root, logger).await;

    let repo_id = options.repo_id.clone();
    let preset_id = options.preset_id.clone();
    let (progress_tx, progress_task) =
        start_progress_forwarder(event_bus, repo_id.clone(), preset_id.clone());

    let blocking_options = BuildBundleBlockingOptions::from(options);
    let build_result = tokio::task::spawn_blocking(move || {
        build_bundle_blocking(
            blocking_options,
            built_at_iso,
            timestamp,
            git_info,
            progress_tx,
        )
    })
    .await;

    let build_result = match build_result {
        Ok(Ok(result)) => result,
        Ok(Err(err)) => {
            let _ = progress_task.await;
            return Err(err);
        }
        Err(err) => {
            let _ = progress_task.await;
            return Err(AgentError::internal(format!("Bundle task failed: {err}")));
        }
    };

    let _ = progress_task.await;

    event_bus.broadcast_event(AgentEvent::BundleBuildProgress(BundleBuildProgressEvent {
        repo_id: build_result.repo_id.clone(),
        preset_id: build_result.preset_id.clone(),
        phase: "finalizing".to_string(),
        files_done: build_result.file_count,
        files_total: build_result.file_count,
        current_file: None,
        current_bytes_done: None,
        current_bytes_total: None,
        bytes_done_total_best_effort: None,
    }));

    Ok(BuildBundleResult {
        host_path: build_result.host_path.clone(),
        wsl_path: build_result.wsl_path.clone(),
        alias_host_path: build_result.host_path,
        bytes: build_result.bytes,
        file_count: build_result.file_count,
        built_at_iso: build_result.built_at_iso,
    })
}
