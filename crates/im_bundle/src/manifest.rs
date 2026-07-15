// Path: crates/im_bundle/src/manifest.rs
// Description: Bundle manifest structure and serialization

use serde::Serialize;

use crate::git_capture::BundleGitCapture;
use crate::global_excludes_summary::normalized_global_excludes_summary;
use crate::plan::{BundleSelection, GlobalExcludes};

pub const BUNDLE_FORMAT_VERSION: u32 = 2;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleManifest {
    pub bundle_format_version: u32,
    pub generated_at: String,
    pub repo_id: String,
    pub repo_root: String,
    pub preset_id: String,
    pub preset_name: String,
    pub selection: ManifestSelection,
    pub effective_global_excludes: GlobalExcludes,
    pub git: BundleGitCapture,
    pub file_count: u64,
    pub total_bytes_best_effort: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestSelection {
    pub include_root: bool,
    pub top_level_dirs_included: Vec<String>,
    pub included_subdirs: Vec<String>,
    pub excluded_subdirs: Vec<String>,
    pub excluded_files: Vec<String>,
}

pub fn build_manifest(
    generated_at: &str,
    repo_id: &str,
    repo_root: &str,
    preset_id: &str,
    preset_name: &str,
    selection: &BundleSelection,
    global_excludes: &GlobalExcludes,
    top_level_dirs_included: &[String],
    git: &BundleGitCapture,
    file_count: u64,
    total_bytes_best_effort: u64,
) -> BundleManifest {
    BundleManifest {
        bundle_format_version: BUNDLE_FORMAT_VERSION,
        generated_at: generated_at.to_string(),
        repo_id: repo_id.to_string(),
        repo_root: repo_root.to_string(),
        preset_id: preset_id.to_string(),
        preset_name: preset_name.to_string(),
        selection: ManifestSelection {
            include_root: selection.include_root,
            top_level_dirs_included: top_level_dirs_included.to_vec(),
            included_subdirs: selection.included_subdirs.clone(),
            excluded_subdirs: selection.excluded_subdirs.clone(),
            excluded_files: selection.excluded_files.clone(),
        },
        effective_global_excludes: normalized_global_excludes_summary(global_excludes),
        git: git.clone(),
        file_count,
        total_bytes_best_effort,
    }
}
