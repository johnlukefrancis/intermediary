// Path: src-tauri/src/lib/config/types.rs
// Description: Persisted configuration types for Intermediary

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Current config schema version
pub const CONFIG_VERSION: u32 = 15;

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
    /// Custom output folder override (Windows path)
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
}

impl Default for PersistedConfig {
    fn default() -> Self {
        Self {
            config_version: CONFIG_VERSION,
            agent_host: "127.0.0.1".to_string(),
            agent_port: 3141,
            agent_auto_start: default_agent_auto_start(),
            agent_distro: None,
            auto_stage_global: true,
            repos: default_repos(),
            recent_files_limit: default_recent_files_limit(),
            ui_state: UiState::default(),
            bundle_selections: HashMap::new(),
            global_excludes: GlobalExcludes::default(),
            output_windows_root: None,
            tab_themes: HashMap::new(),
            starred_files: HashMap::new(),
            theme_mode: ThemeMode::default(),
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
}

/// Global excludes for bundle building (not per-repo, not per-preset)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
    /// Blue-light filter mode with amber/sepia undertones
    Warm,
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
    /// Absolute WSL path to repo root
    pub wsl_path: String,
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

/// Validate config invariants before saving
pub fn validate_config(config: &PersistedConfig) -> Result<(), String> {
    if config.config_version == 0 || config.config_version > CONFIG_VERSION {
        return Err(format!(
            "config_version {} is not supported (max {CONFIG_VERSION})",
            config.config_version
        ));
    }

    if config.agent_host.trim().is_empty() {
        return Err("agent_host must not be empty".to_string());
    }

    if config.agent_port < 1024 {
        return Err("agent_port must be >= 1024".to_string());
    }

    if let Some(distro) = &config.agent_distro {
        if distro.trim().is_empty() {
            return Err("agent_distro must not be empty when provided".to_string());
        }
    }

    if config.recent_files_limit < 25 || config.recent_files_limit > 2000 {
        return Err(format!(
            "recent_files_limit must be 25-2000, got {}",
            config.recent_files_limit
        ));
    }

    let mut repo_ids = HashSet::new();
    for repo in &config.repos {
        validate_non_empty(&repo.repo_id, "repo.repo_id")?;
        validate_non_empty(&repo.label, "repo.label")?;
        validate_non_empty(&repo.wsl_path, "repo.wsl_path")?;

        if !repo_ids.insert(repo.repo_id.as_str()) {
            return Err(format!("duplicate repo_id: {}", repo.repo_id));
        }

        for preset in &repo.bundle_presets {
            validate_non_empty(&preset.preset_id, "bundle_preset.preset_id")?;
            validate_non_empty(&preset.preset_name, "bundle_preset.preset_name")?;
        }
    }

    // Validate tabTheme accent colors
    for (tab_key, theme) in &config.tab_themes {
        validate_accent_hex(&theme.accent_hex, tab_key)?;
        if let Some(texture_id) = &theme.texture_id {
            if texture_id.trim().is_empty() {
                return Err(format!(
                    "tabTheme texture_id for {tab_key} must not be empty when provided"
                ));
            }
        }
    }

    Ok(())
}

fn validate_non_empty(value: &str, field: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{field} must not be empty"));
    }
    Ok(())
}

/// Regex for #RRGGBB hex color format
static ACCENT_HEX_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^#[0-9A-Fa-f]{6}$").expect("valid regex"));

fn validate_accent_hex(value: &str, tab_key: &str) -> Result<(), String> {
    if !ACCENT_HEX_REGEX.is_match(value) {
        return Err(format!(
            "tabTheme accent_hex for {tab_key} must be #RRGGBB format, got: {value}"
        ));
    }
    Ok(())
}

fn default_repos() -> Vec<RepoConfig> {
    Vec::new()
}

fn default_agent_auto_start() -> bool {
    true
}

fn default_recent_files_limit() -> u32 {
    200
}

#[cfg(test)]
mod tests {
    use super::CONFIG_VERSION;
    use regex::Regex;

    #[test]
    fn ts_config_version_matches_rust() {
        let contents = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../app/src/shared/config/version.ts"
        ));
        let regex = Regex::new(r"CONFIG_VERSION\\s*=\\s*(\\d+)").expect("valid regex");
        let caps = regex
            .captures(contents)
            .expect("CONFIG_VERSION not found in version.ts");
        let ts_version: u32 = caps[1]
            .parse()
            .expect("CONFIG_VERSION in version.ts must be a number");
        assert_eq!(
            ts_version, CONFIG_VERSION,
            "TS CONFIG_VERSION {ts_version} must match Rust CONFIG_VERSION {CONFIG_VERSION}"
        );
    }
}
