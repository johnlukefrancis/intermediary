// Path: src-tauri/src/lib/config/types/model.rs
// Description: Supporting persisted configuration model types

use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

use super::PersistedConfig;

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
    /// Path patterns to exclude (e.g. ".huggingface/", "wandb/")
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
    /// Subdirectories explicitly included despite matching a default directory exclude
    #[serde(default)]
    pub included_subdirs: Vec<String>,
    /// Subdirectories to exclude (e.g. "TriangleRain/Assets")
    #[serde(default)]
    pub excluded_subdirs: Vec<String>,
    /// Repo-relative files to exclude from selected roots/directories
    #[serde(default)]
    pub excluded_files: Vec<String>,
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
    /// Muted warm light mode - parchment/linen aesthetic
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

pub(super) fn deserialize_ui_mode_or_default<'de, D>(deserializer: D) -> Result<UiMode, D::Error>
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

/// Legacy starred files for a single repo
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
