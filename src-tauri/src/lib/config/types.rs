// Path: src-tauri/src/lib/config/types.rs
// Description: Persisted configuration types for Intermediary

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

mod model;
mod ui_state;
mod validation;
use model::deserialize_ui_mode_or_default;
pub use model::{
    clamp_window_bounds, default_window_bounds_for_mode, resolve_window_bounds_for_mode,
    BundlePreset, BundleSelection, GlobalExcludes, LoadConfigResult, RepoConfig, RepoRoot,
    StarredFilesEntry, TabTheme, ThemeMode, UiMode, MAX_WINDOW_HEIGHT, MAX_WINDOW_WIDTH,
    MIN_WINDOW_HEIGHT, MIN_WINDOW_WIDTH,
};
pub use ui_state::{UiState, UiWindowBounds};
pub use validation::validate_config;

/// Current config schema version
pub const CONFIG_VERSION: u32 = 26;

/// Top-level persisted configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedConfig {
    /// Schema version for migrations
    pub config_version: u32,
    /// Hostname for agent WebSocket connection
    pub agent_host: String,
    /// Port for agent WebSocket connection
    pub agent_port: u16,
    /// Auto-start the WSL agent on app launch
    #[serde(default = "default_agent_auto_start")]
    pub agent_auto_start: bool,
    /// Optional WSL distro override for agent launch
    #[serde(default)]
    pub agent_distro: Option<String>,
    /// Global default for auto-staging
    pub auto_stage_global: bool,
    /// Configured repositories
    pub repos: Vec<RepoConfig>,
    /// Maximum recent files to track per repo (25-2000)
    #[serde(default = "default_recent_files_limit")]
    pub recent_files_limit: u32,
    /// Remembered UI state
    pub ui_state: UiState,
    /// Bundle selections per repo/preset
    pub bundle_selections: HashMap<String, HashMap<String, BundleSelection>>,
    /// Global bundle excludes (extensions and patterns)
    #[serde(default)]
    pub global_excludes: GlobalExcludes,
    /// Global classification excludes (used by file feeds only)
    #[serde(default)]
    pub classification_excludes: GlobalExcludes,
    /// Custom output folder override (host-native absolute path)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_windows_root: Option<String>,
    /// Per-tab accent colors, keyed by tabKey
    #[serde(default)]
    pub tab_themes: HashMap<String, TabTheme>,
    /// Legacy starred files per repo
    #[serde(default)]
    pub starred_files: HashMap<String, StarredFilesEntry>,
    /// Global theme mode (dark/warm)
    #[serde(default)]
    pub theme_mode: ThemeMode,
    /// UI density mode (standard/handset)
    #[serde(default, deserialize_with = "deserialize_ui_mode_or_default")]
    pub ui_mode: UiMode,
    /// Global window surface opacity percent (0-100)
    #[serde(default = "default_window_opacity_percent")]
    pub window_opacity_percent: u8,
    /// Global substrate texture intensity percent (0-100)
    #[serde(default = "default_texture_intensity_percent")]
    pub texture_intensity_percent: u8,
}

impl Default for PersistedConfig {
    fn default() -> Self {
        Self {
            config_version: CONFIG_VERSION,
            agent_host: "127.0.0.1".to_string(),
            agent_port: default_agent_port(),
            agent_auto_start: default_agent_auto_start(),
            agent_distro: None,
            auto_stage_global: true,
            repos: default_repos(),
            recent_files_limit: default_recent_files_limit(),
            ui_state: UiState::default(),
            bundle_selections: HashMap::new(),
            global_excludes: GlobalExcludes::default(),
            classification_excludes: GlobalExcludes::default(),
            output_windows_root: None,
            tab_themes: HashMap::new(),
            starred_files: HashMap::new(),
            theme_mode: ThemeMode::default(),
            ui_mode: UiMode::default(),
            window_opacity_percent: default_window_opacity_percent(),
            texture_intensity_percent: default_texture_intensity_percent(),
        }
    }
}

fn default_repos() -> Vec<RepoConfig> {
    Vec::new()
}

fn default_agent_auto_start() -> bool {
    true
}

fn default_agent_port() -> u16 {
    3141
}

fn default_recent_files_limit() -> u32 {
    200
}

fn default_window_opacity_percent() -> u8 {
    100
}

fn default_texture_intensity_percent() -> u8 {
    100
}

#[cfg(test)]
mod tests;
