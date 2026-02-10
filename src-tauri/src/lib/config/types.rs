// Path: src-tauri/src/lib/config/types.rs
// Description: Persisted configuration types for Intermediary

use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

mod validation;
pub use validation::validate_config;

/// Current config schema version
pub const CONFIG_VERSION: u32 = 21;

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
    /// Global classification excludes (used by Docs/Code panes only)
    #[serde(default)]
    pub classification_excludes: GlobalExcludes,
    /// Custom output folder override (host-native absolute path)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_windows_root: Option<String>,
    /// Per-tab accent colors, keyed by tabKey
    #[serde(default)]
    pub tab_themes: HashMap<String, TabTheme>,
    /// Starred files per repo
    #[serde(default)]
    pub starred_files: HashMap<String, StarredFilesEntry>,
    /// Global theme mode (dark/warm)
    #[serde(default)]
    pub theme_mode: ThemeMode,
    /// UI density mode (standard/handset)
    #[serde(default, deserialize_with = "deserialize_ui_mode_or_default")]
    pub ui_mode: UiMode,
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
        }
    }
}

/// Remembered UI choices
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiState {
    /// Last active repo (by repoId)
    pub last_active_tab_id: Option<String>,
    /// Last active repo per group (groupId -> repoId)
    #[serde(default)]
    pub last_active_group_repo_ids: HashMap<String, String>,
    /// Remembered window bounds by mode key (standard/handset)
    #[serde(default)]
    pub window_bounds_by_mode: HashMap<String, UiWindowBounds>,
}

/// Window bounds persisted for a specific mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiWindowBounds {
    pub width: u32,
    pub height: u32,
}

/// Global excludes for bundle building (not per-repo, not per-preset)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalExcludes {
    /// Directory names to exclude (exact match)
    #[serde(default)]
    pub dir_names: Vec<String>,
    /// Directory name suffixes to exclude (e.g. ".egg-info")
    #[serde(default)]
    pub dir_suffixes: Vec<String>,
    /// File names to exclude (exact match)
    #[serde(default)]
    pub file_names: Vec<String>,
    /// File extensions to exclude (e.g. ".safetensors", ".ckpt")
    #[serde(default)]
    pub extensions: Vec<String>,
    /// Path patterns to exclude (e.g. "models/", "checkpoints/")
    #[serde(default)]
    pub patterns: Vec<String>,
}

impl Default for GlobalExcludes {
    fn default() -> Self {
        let recommended = im_bundle::global_excludes::recommended_global_excludes();
        Self {
            dir_names: recommended.dir_names,
            dir_suffixes: recommended.dir_suffixes,
            file_names: recommended.file_names,
            extensions: recommended.extensions,
            patterns: recommended.patterns,
        }
    }
}

/// Bundle selection state for a preset
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleSelection {
    /// Whether to include root-level files
    pub include_root: bool,
    /// Selected top-level directories
    pub top_level_dirs: Vec<String>,
    /// Subdirectories to exclude (e.g. "TriangleRain/Assets")
    #[serde(default)]
    pub excluded_subdirs: Vec<String>,
}

/// Per-tab theme configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TabTheme {
    /// Accent color in #RRGGBB format
    pub accent_hex: String,
    /// Optional texture id (from app/assets/textures)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture_id: Option<String>,
}

/// Global theme mode (color temperature)
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    /// Standard dark mode with blue undertones
    #[default]
    Dark,
    /// Muted warm light mode — parchment/linen aesthetic
    Light,
    /// Blue-light filter mode with amber/sepia undertones
    Warm,
}

/// UI density mode
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UiMode {
    #[default]
    Standard,
    Handset,
}

impl UiMode {
    pub fn as_key(self) -> &'static str {
        match self {
            UiMode::Standard => "standard",
            UiMode::Handset => "handset",
        }
    }
}

pub const MIN_WINDOW_WIDTH: u32 = 360;
pub const MIN_WINDOW_HEIGHT: u32 = 500;
pub const MAX_WINDOW_WIDTH: u32 = 8192;
pub const MAX_WINDOW_HEIGHT: u32 = 8192;

pub fn clamp_window_bounds(bounds: UiWindowBounds) -> UiWindowBounds {
    UiWindowBounds {
        width: bounds.width.clamp(MIN_WINDOW_WIDTH, MAX_WINDOW_WIDTH),
        height: bounds.height.clamp(MIN_WINDOW_HEIGHT, MAX_WINDOW_HEIGHT),
    }
}

pub fn default_window_bounds_for_mode(mode: UiMode) -> UiWindowBounds {
    match mode {
        UiMode::Standard => UiWindowBounds {
            width: 1200,
            height: 800,
        },
        UiMode::Handset => UiWindowBounds {
            width: 420,
            height: 660,
        },
    }
}

pub fn resolve_window_bounds_for_mode(config: &PersistedConfig, mode: UiMode) -> UiWindowBounds {
    let mode_key = mode.as_key();
    let bounds = config
        .ui_state
        .window_bounds_by_mode
        .get(mode_key)
        .copied()
        .unwrap_or_else(|| default_window_bounds_for_mode(mode));
    clamp_window_bounds(bounds)
}

fn deserialize_ui_mode_or_default<'de, D>(deserializer: D) -> Result<UiMode, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = Option::<String>::deserialize(deserializer)?;
    Ok(match raw.as_deref() {
        Some("standard") => UiMode::Standard,
        Some("compact") => UiMode::Standard,
        Some("handset") => UiMode::Handset,
        _ => UiMode::Standard,
    })
}

/// Starred files for a single repo
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StarredFilesEntry {
    #[serde(default)]
    pub docs: Vec<String>,
    #[serde(default)]
    pub code: Vec<String>,
}

/// Configuration for a single repository
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoConfig {
    /// Unique identifier for this repo
    pub repo_id: String,
    /// Display name in UI (shown in dropdown for grouped repos)
    pub label: String,
    /// Repo root authority (WSL-native or host-native)
    pub root: RepoRoot,
    /// Optional group ID - repos with same groupId share a tab with dropdown
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    /// Group display name (shown as tab label for grouped repos)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_label: Option<String>,
    /// Whether to auto-stage changes
    pub auto_stage: bool,
    /// Globs for docs classification
    pub docs_globs: Vec<String>,
    /// Globs for code classification
    pub code_globs: Vec<String>,
    /// Globs to ignore
    pub ignore_globs: Vec<String>,
    /// Bundle presets
    pub bundle_presets: Vec<BundlePreset>,
}

/// Path-native repository root
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RepoRoot {
    /// Linux path within WSL (e.g. /home/john/code/repo)
    Wsl { path: String },
    /// Host-native path (Windows path on Windows; POSIX path on macOS/Linux).
    #[serde(alias = "windows")]
    Host { path: String },
}

impl RepoRoot {
    pub fn path(&self) -> &str {
        match self {
            RepoRoot::Wsl { path } | RepoRoot::Host { path } => path,
        }
    }
}

/// Bundle preset configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundlePreset {
    /// Unique preset identifier
    pub preset_id: String,
    /// Display name
    pub preset_name: String,
    /// Include root-level files by default
    pub include_root: bool,
    /// Default top-level directories
    pub top_level_dirs: Vec<String>,
}

/// Result of loading config
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadConfigResult {
    /// The loaded configuration
    pub config: PersistedConfig,
    /// True if config was freshly created (didn't exist)
    pub was_created: bool,
    /// True if migration was applied
    pub migration_applied: bool,
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
    40
}

#[cfg(test)]
mod tests;
