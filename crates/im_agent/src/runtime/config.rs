// Path: crates/im_agent/src/runtime/config.rs
// Description: Minimal app configuration structures for the agent runtime

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub agent_host: Option<String>,
    pub agent_port: Option<u16>,
    #[serde(default = "default_true")]
    pub auto_stage_global: bool,
    #[serde(default = "default_recent_files_limit")]
    pub recent_files_limit: usize,
    #[serde(default)]
    pub repos: Vec<RepoConfig>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoConfig {
    pub repo_id: String,
    pub wsl_path: String,
    #[serde(default = "default_true")]
    pub auto_stage: bool,
    #[serde(default)]
    pub docs_globs: Vec<String>,
    #[serde(default)]
    pub code_globs: Vec<String>,
    #[serde(default)]
    pub ignore_globs: Vec<String>,
    #[serde(default = "default_bundle_presets")]
    pub bundle_presets: Vec<BundlePreset>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundlePreset {
    pub preset_id: String,
    pub preset_name: String,
    #[serde(default = "default_true")]
    pub include_root: bool,
    #[serde(default)]
    pub top_level_dirs: Vec<String>,
}

fn default_true() -> bool {
    true
}

fn default_recent_files_limit() -> usize {
    200
}

fn default_bundle_presets() -> Vec<BundlePreset> {
    vec![BundlePreset {
        preset_id: "context".to_string(),
        preset_name: "Context".to_string(),
        include_root: true,
        top_level_dirs: Vec::new(),
    }]
}
