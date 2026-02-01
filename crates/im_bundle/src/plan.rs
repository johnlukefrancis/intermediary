// Path: crates/im_bundle/src/plan.rs
// Description: Bundle plan schema and loader for im_bundle_cli

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{BundleError, Result};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BundleSelection {
    pub include_root: bool,
    pub top_level_dirs: Vec<String>,
    pub excluded_subdirs: Vec<String>,
}

fn default_true() -> bool {
    true
}

/// Global exclude preset toggles.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GlobalExcludePresets {
    /// Exclude ML artifacts (model weights + experiment caches).
    #[serde(default = "default_true")]
    pub ml_artifacts: bool,
}

impl Default for GlobalExcludePresets {
    fn default() -> Self {
        Self { ml_artifacts: true }
    }
}

/// Global excludes for bundle building (user-configurable, supplements hardcoded excludes)
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct GlobalExcludes {
    /// Preset toggles for common large artifacts.
    #[serde(default)]
    pub presets: GlobalExcludePresets,
    /// File extensions to exclude (e.g. ".safetensors", ".ckpt")
    #[serde(default)]
    pub extensions: Vec<String>,
    /// Path patterns to exclude (e.g. "models/", "checkpoints/")
    #[serde(default)]
    pub patterns: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BundleGitInfo {
    pub head_sha: Option<String>,
    pub short_sha: Option<String>,
    pub branch: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundlePlan {
    pub output_path: PathBuf,
    pub repo_root: PathBuf,
    pub repo_id: String,
    pub preset_id: String,
    pub preset_name: String,
    pub selection: BundleSelection,
    pub git: BundleGitInfo,
    pub built_at_iso: String,
    /// User-configurable global excludes (supplements hardcoded excludes)
    #[serde(default)]
    pub global_excludes: GlobalExcludes,
}

impl BundlePlan {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|source| {
            BundleError::PlanReadFailed {
                path: path.to_path_buf(),
                source,
            }
        })?;
        let plan: BundlePlan = serde_json::from_str(&content)?;
        plan.validate()?;
        Ok(plan)
    }

    fn validate(&self) -> Result<()> {
        if self.repo_id.trim().is_empty() {
            return Err(BundleError::InvalidPlan("repoId is required".to_string()));
        }
        if self.preset_id.trim().is_empty() {
            return Err(BundleError::InvalidPlan("presetId is required".to_string()));
        }
        if self.preset_name.trim().is_empty() {
            return Err(BundleError::InvalidPlan("presetName is required".to_string()));
        }
        for dir in &self.selection.top_level_dirs {
            let trimmed = dir.trim();
            if trimmed.is_empty() {
                return Err(BundleError::InvalidPlan(
                    "topLevelDirs entries cannot be empty".to_string(),
                ));
            }
            if trimmed == "." || trimmed == ".." || trimmed.contains('/') || trimmed.contains('\\')
            {
                return Err(BundleError::InvalidPlan(format!(
                    "topLevelDirs must be simple directory names: {trimmed}"
                )));
            }
            if trimmed.split('/').any(|part| part == "..") {
                return Err(BundleError::InvalidPlan(format!(
                    "topLevelDirs cannot contain '..': {trimmed}"
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn parse_plan() {
        let json = r#"{
            "outputPath": "/tmp/out.zip",
            "repoRoot": "/tmp/repo",
            "repoId": "intermediary",
            "presetId": "full",
            "presetName": "Full",
            "selection": {
              "includeRoot": true,
              "topLevelDirs": ["app"],
              "excludedSubdirs": []
            },
            "git": {"headSha": null, "shortSha": null, "branch": null},
            "builtAtIso": "2026-01-31T00:00:00Z"
        }"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(json.as_bytes()).unwrap();

        let plan = BundlePlan::load(file.path()).unwrap();
        assert_eq!(plan.repo_id, "intermediary");
        assert!(plan.selection.include_root);
    }
}
