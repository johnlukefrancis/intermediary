// Path: crates/im_agent/src/bundles/bundle_builder_blocking.rs
// Description: Blocking bundle build steps and filesystem operations

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};
use tokio::sync::mpsc;

use im_bundle::progress::ProgressMessage;
use im_bundle::progress_sink::CallbackProgressSink;
use im_bundle::writer::write_bundle_with_progress;
use im_bundle::BundlePlan;

use crate::error::AgentError;
use crate::protocol::{BundleSelection, GlobalExcludes};
use crate::staging::{PathBridgeConfig, StagingLayout, StagingRootKind};

use super::git_info::GitInfo;

pub(crate) struct BuildBundleBlockingOptions {
    pub(crate) repo_id: String,
    pub(crate) repo_root: String,
    pub(crate) preset_id: String,
    pub(crate) preset_name: String,
    pub(crate) selection: BundleSelection,
    pub(crate) staging: PathBridgeConfig,
    pub(crate) staging_kind: StagingRootKind,
    pub(crate) global_excludes: Option<GlobalExcludes>,
}

pub(crate) struct BlockingBundleResult {
    pub(crate) host_path: String,
    pub(crate) wsl_path: Option<String>,
    pub(crate) bytes: u64,
    pub(crate) file_count: u64,
    pub(crate) built_at_iso: String,
}

pub(crate) fn build_bundle_blocking(
    options: BuildBundleBlockingOptions,
    built_at_iso: String,
    timestamp: String,
    git_info: GitInfo,
    progress_tx: mpsc::UnboundedSender<ProgressMessage>,
) -> Result<BlockingBundleResult, AgentError> {
    let layout = StagingLayout::from_config(&options.staging, options.staging_kind)?;
    let output_dir = layout.bundles_dir(&options.repo_id, &options.preset_id);
    std::fs::create_dir_all(&output_dir)
        .map_err(|err| AgentError::internal(format!("Failed to create bundle directory: {err}")))?;

    let base_name = format!("{}_{}_{}", options.repo_id, options.preset_id, timestamp);
    let file_name = match git_info.short_sha.as_deref() {
        Some(short_sha) if !short_sha.trim().is_empty() => {
            format!("{base_name}_{short_sha}.zip")
        }
        _ => format!("{base_name}.zip"),
    };

    let final_path = output_dir.join(file_name);
    let temp_path = temp_path_for(&final_path);

    let sink = CallbackProgressSink::new(move |message| {
        let _ = progress_tx.send(message);
    });

    let plan = build_plan(&options, &temp_path, &built_at_iso, git_info);

    let bundle_result = match write_bundle_with_progress(&plan, Box::new(sink)) {
        Ok(result) => result,
        Err(err) => {
            let _ = std::fs::remove_file(&temp_path);
            return Err(AgentError::new("BUNDLE_BUILD_FAILED", err.to_string()));
        }
    };

    if let Err(err) = std::fs::rename(&temp_path, &final_path) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(AgentError::internal(format!(
            "Failed to finalize bundle: {err}"
        )));
    }

    cleanup_older_bundles(
        &output_dir,
        &options.repo_id,
        &options.preset_id,
        &final_path,
    );

    let bundle_paths = layout.path_views_for_runtime_path(&final_path)?;

    Ok(BlockingBundleResult {
        host_path: bundle_paths.host_path,
        wsl_path: bundle_paths.wsl_path,
        bytes: bundle_result.bytes_written,
        file_count: bundle_result.file_count,
        built_at_iso,
    })
}

pub(crate) fn format_timestamp(date: DateTime<Utc>) -> String {
    date.format("%Y%m%d_%H%M%S").to_string()
}

pub(crate) fn cleanup_older_bundles(
    bundle_dir: &Path,
    repo_id: &str,
    preset_id: &str,
    keep_path: &Path,
) {
    let prefix = format!("{repo_id}_{preset_id}_");
    let entries = match std::fs::read_dir(bundle_dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        let path = entry.path();
        if path == keep_path {
            continue;
        }
        if file_name.starts_with(&prefix) && file_name.ends_with(".zip") {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

fn build_plan(
    options: &BuildBundleBlockingOptions,
    output_path: &Path,
    built_at_iso: &str,
    git_info: GitInfo,
) -> BundlePlan {
    BundlePlan {
        output_path: output_path.to_path_buf(),
        repo_root: PathBuf::from(&options.repo_root),
        repo_id: options.repo_id.clone(),
        preset_id: options.preset_id.clone(),
        preset_name: options.preset_name.clone(),
        selection: im_bundle::plan::BundleSelection {
            include_root: options.selection.include_root,
            top_level_dirs: options.selection.top_level_dirs.clone(),
            excluded_subdirs: options.selection.excluded_subdirs.clone(),
        },
        git: im_bundle::plan::BundleGitInfo {
            head_sha: git_info.head_sha,
            short_sha: git_info.short_sha,
            branch: git_info.branch,
        },
        built_at_iso: built_at_iso.to_string(),
        global_excludes: map_global_excludes(options.global_excludes.as_ref()),
    }
}

fn map_global_excludes(excludes: Option<&GlobalExcludes>) -> im_bundle::plan::GlobalExcludes {
    let defaults = im_bundle::plan::GlobalExcludes::default();
    match excludes {
        Some(excludes) => im_bundle::plan::GlobalExcludes {
            dir_names: merge_exclude_values(defaults.dir_names, &excludes.dir_names),
            dir_suffixes: merge_exclude_values(defaults.dir_suffixes, &excludes.dir_suffixes),
            file_names: merge_exclude_values(defaults.file_names, &excludes.file_names),
            extensions: merge_exclude_values(defaults.extensions, &excludes.extensions),
            patterns: merge_exclude_values(defaults.patterns, &excludes.patterns),
        },
        None => defaults,
    }
}

fn merge_exclude_values(mut baseline: Vec<String>, user: &[String]) -> Vec<String> {
    for item in user {
        if !baseline.contains(item) {
            baseline.push(item.clone());
        }
    }
    baseline
}

fn temp_path_for(path: &Path) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "bundle".to_string());
    let temp_name = format!("{file_name}.{suffix}.tmp");
    path.with_file_name(temp_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_global_excludes_when_missing() {
        let excludes = map_global_excludes(None);
        assert!(excludes.dir_names.iter().any(|name| name == "node_modules"));
        assert!(excludes.dir_names.iter().any(|name| name == "target"));
    }

    #[test]
    fn enforces_recommended_excludes_when_user_sends_empty() {
        let empty = GlobalExcludes {
            dir_names: vec![],
            dir_suffixes: vec![],
            file_names: vec![],
            extensions: vec![],
            patterns: vec![],
        };
        let excludes = map_global_excludes(Some(&empty));
        assert!(excludes.dir_names.iter().any(|name| name == "target"));
        assert!(excludes.dir_names.iter().any(|name| name == "node_modules"));
        assert!(excludes.dir_names.iter().any(|name| name == ".git"));
        assert!(excludes.file_names.iter().any(|name| name == ".ds_store"));
        assert!(!excludes.extensions.is_empty());
        assert!(!excludes.patterns.is_empty());
    }

    #[test]
    fn merges_user_excludes_with_recommended_baseline() {
        let custom = GlobalExcludes {
            dir_names: vec!["my_custom_dir".to_string(), "target".to_string()],
            dir_suffixes: vec![],
            file_names: vec![],
            extensions: vec![".custom_ext".to_string()],
            patterns: vec![],
        };
        let excludes = map_global_excludes(Some(&custom));
        // Recommended are present
        assert!(excludes.dir_names.iter().any(|name| name == "target"));
        assert!(excludes.dir_names.iter().any(|name| name == "node_modules"));
        // User custom is merged
        assert!(excludes
            .dir_names
            .iter()
            .any(|name| name == "my_custom_dir"));
        assert!(excludes.extensions.iter().any(|ext| ext == ".custom_ext"));
        // No duplicates for "target" (appears in both user and recommended)
        let target_count = excludes.dir_names.iter().filter(|n| *n == "target").count();
        assert_eq!(target_count, 1);
    }
}
