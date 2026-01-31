// Path: crates/im_bundle/src/manifest.rs
// Description: Bundle manifest structure and serialization

use serde::Serialize;

use crate::plan::{BundleGitInfo, BundleSelection};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleManifest {
    pub generated_at: String,
    pub repo_id: String,
    pub repo_root: String,
    pub preset_id: String,
    pub preset_name: String,
    pub selection: ManifestSelection,
    pub git: BundleGitInfo,
    pub file_count: u64,
    pub total_bytes_best_effort: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestSelection {
    pub include_root: bool,
    pub top_level_dirs_included: Vec<String>,
    pub excluded_subdirs: Vec<String>,
}

pub fn build_manifest(
    generated_at: &str,
    repo_id: &str,
    repo_root: &str,
    preset_id: &str,
    preset_name: &str,
    selection: &BundleSelection,
    top_level_dirs_included: &[String],
    git: &BundleGitInfo,
    file_count: u64,
    total_bytes_best_effort: u64,
) -> BundleManifest {
    BundleManifest {
        generated_at: generated_at.to_string(),
        repo_id: repo_id.to_string(),
        repo_root: repo_root.to_string(),
        preset_id: preset_id.to_string(),
        preset_name: preset_name.to_string(),
        selection: ManifestSelection {
            include_root: selection.include_root,
            top_level_dirs_included: top_level_dirs_included.to_vec(),
            excluded_subdirs: selection.excluded_subdirs.clone(),
        },
        git: BundleGitInfo {
            head_sha: git.head_sha.clone(),
            short_sha: git.short_sha.clone(),
            branch: git.branch.clone(),
        },
        file_count,
        total_bytes_best_effort,
    }
}
