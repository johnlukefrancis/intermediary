// Path: crates/im_agent/src/bundles/bundle_builder.rs
// Description: Bundle build orchestration using the im_bundle library

use chrono::Utc;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::error::AgentError;
use crate::logging::Logger;
use crate::protocol::{BundleSelection, GlobalExcludes};
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
    pub build_id: String,
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

#[derive(Clone, PartialEq, Eq, Hash)]
struct BuildLockKey {
    repo_id: String,
    preset_id: String,
}

static ACTIVE_BUNDLE_BUILDS: OnceLock<Mutex<HashMap<BuildLockKey, ActiveBundleBuild>>> =
    OnceLock::new();

struct ActiveBundleBuild {
    build_id: String,
    cancel_token: im_bundle::cancel::BundleCancelToken,
}

struct BuildLockGuard {
    key: BuildLockKey,
    build_id: String,
}

impl Drop for BuildLockGuard {
    fn drop(&mut self) {
        if let Ok(mut active) = active_bundle_builds().lock() {
            if active
                .get(&self.key)
                .is_some_and(|active| active.build_id == self.build_id)
            {
                active.remove(&self.key);
            }
        }
    }
}

fn active_bundle_builds() -> &'static Mutex<HashMap<BuildLockKey, ActiveBundleBuild>> {
    ACTIVE_BUNDLE_BUILDS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn build_lock_key(repo_id: &str, preset_id: &str) -> BuildLockKey {
    BuildLockKey {
        repo_id: repo_id.to_string(),
        preset_id: preset_id.to_string(),
    }
}

fn acquire_build_lock(
    repo_id: &str,
    preset_id: &str,
    build_id: &str,
    cancel_token: im_bundle::cancel::BundleCancelToken,
) -> Result<BuildLockGuard, AgentError> {
    let key = build_lock_key(repo_id, preset_id);
    let mut active = active_bundle_builds()
        .lock()
        .map_err(|_| AgentError::internal("Bundle build lock state is unavailable"))?;
    if active.contains_key(&key) {
        return Err(AgentError::new(
            "BUNDLE_BUILD_IN_PROGRESS",
            format!("Bundle build already in progress for {repo_id}/{preset_id}"),
        ));
    }
    active.insert(
        key.clone(),
        ActiveBundleBuild {
            build_id: build_id.to_string(),
            cancel_token,
        },
    );
    Ok(BuildLockGuard {
        key,
        build_id: build_id.to_string(),
    })
}

pub fn cancel_bundle_build(repo_id: &str, preset_id: &str, build_id: &str) -> bool {
    let key = build_lock_key(repo_id, preset_id);
    let active = match active_bundle_builds().lock() {
        Ok(active) => active,
        Err(_) => return false,
    };
    let Some(active_build) = active.get(&key) else {
        return false;
    };
    if active_build.build_id != build_id {
        return false;
    }
    active_build.cancel_token.cancel();
    true
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
    let cancel_token = im_bundle::cancel::BundleCancelToken::new();
    let _build_lock = acquire_build_lock(
        &options.repo_id,
        &options.preset_id,
        &options.build_id,
        cancel_token.clone(),
    )?;
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
            cancel_token,
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

    Ok(BuildBundleResult {
        host_path: build_result.host_path.clone(),
        wsl_path: build_result.wsl_path.clone(),
        alias_host_path: build_result.host_path,
        bytes: build_result.bytes,
        file_count: build_result.file_count,
        built_at_iso: build_result.built_at_iso,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_duplicate_build_lock_for_same_repo_and_preset() {
        let first = acquire_build_lock(
            "repo_duplicate_lock",
            "preset_duplicate_lock",
            "build_1",
            im_bundle::cancel::BundleCancelToken::new(),
        )
        .expect("first lock");
        let err = match acquire_build_lock(
            "repo_duplicate_lock",
            "preset_duplicate_lock",
            "build_2",
            im_bundle::cancel::BundleCancelToken::new(),
        ) {
            Ok(_) => panic!("second lock should fail"),
            Err(err) => err,
        };
        assert_eq!(err.code(), "BUNDLE_BUILD_IN_PROGRESS");
        drop(first);
        acquire_build_lock(
            "repo_duplicate_lock",
            "preset_duplicate_lock",
            "build_3",
            im_bundle::cancel::BundleCancelToken::new(),
        )
        .expect("lock should release after drop");
    }

    #[test]
    fn distinct_ids_with_delimiter_do_not_collide() {
        let first = acquire_build_lock(
            "a",
            "b::c",
            "build_1",
            im_bundle::cancel::BundleCancelToken::new(),
        )
        .expect("first lock");
        let second = acquire_build_lock(
            "a::b",
            "c",
            "build_2",
            im_bundle::cancel::BundleCancelToken::new(),
        )
        .expect("distinct repo/preset pair should not collide");
        drop(second);
        drop(first);
    }

    #[test]
    fn cancels_only_matching_active_build_id() {
        let token = im_bundle::cancel::BundleCancelToken::new();
        let guard = acquire_build_lock(
            "repo_cancel_lock",
            "preset_cancel_lock",
            "build_1",
            token.clone(),
        )
        .expect("lock");

        assert!(!cancel_bundle_build(
            "repo_cancel_lock",
            "preset_cancel_lock",
            "build_2"
        ));
        assert!(!token.is_cancelled());
        assert!(cancel_bundle_build(
            "repo_cancel_lock",
            "preset_cancel_lock",
            "build_1"
        ));
        assert!(token.is_cancelled());

        drop(guard);
    }
}
