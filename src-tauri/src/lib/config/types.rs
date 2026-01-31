// Path: src-tauri/src/lib/config/types.rs
// Description: Persisted configuration types for Intermediary

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Current config schema version
pub const CONFIG_VERSION: u32 = 2;

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
    /// Global default for auto-staging
    pub auto_stage_global: bool,
    /// Configured repositories
    pub repos: Vec<RepoConfig>,
    /// Remembered UI state
    pub ui_state: UiState,
    /// Bundle selections per repo/preset
    pub bundle_selections: HashMap<String, HashMap<String, BundleSelection>>,
}

impl Default for PersistedConfig {
    fn default() -> Self {
        Self {
            config_version: CONFIG_VERSION,
            agent_host: "localhost".to_string(),
            agent_port: 3141,
            auto_stage_global: true,
            repos: default_repos(),
            ui_state: UiState::default(),
            bundle_selections: HashMap::new(),
        }
    }
}

/// Remembered UI choices
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiState {
    /// Last active tab ID
    pub last_active_tab_id: Option<String>,
    /// Last selected Triangle Rain worktree
    pub last_triangle_rain_worktree_id: Option<String>,
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

/// Configuration for a single repository
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoConfig {
    /// Unique identifier for this repo
    pub repo_id: String,
    /// Display name in UI
    pub label: String,
    /// Absolute WSL path to repo root
    pub wsl_path: String,
    /// Tab this repo belongs to
    pub tab_id: String,
    /// Optional worktree ID (for Triangle Rain)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worktree_id: Option<String>,
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

    let mut repo_ids = HashSet::new();
    for repo in &config.repos {
        validate_non_empty(&repo.repo_id, "repo.repo_id")?;
        validate_non_empty(&repo.label, "repo.label")?;
        validate_non_empty(&repo.wsl_path, "repo.wsl_path")?;
        validate_non_empty(&repo.tab_id, "repo.tab_id")?;

        if !repo_ids.insert(repo.repo_id.as_str()) {
            return Err(format!("duplicate repo_id: {}", repo.repo_id));
        }

        for preset in &repo.bundle_presets {
            validate_non_empty(&preset.preset_id, "bundle_preset.preset_id")?;
            validate_non_empty(&preset.preset_name, "bundle_preset.preset_name")?;
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

fn default_repos() -> Vec<RepoConfig> {
    vec![
        RepoConfig {
            repo_id: "textureportal".to_string(),
            label: "TexturePortal".to_string(),
            wsl_path: "/home/johnf/code/textureportal".to_string(),
            tab_id: "texture-portal".to_string(),
            worktree_id: None,
            auto_stage: true,
            docs_globs: default_docs_globs(),
            code_globs: default_code_globs(),
            ignore_globs: default_ignore_globs(),
            bundle_presets: vec![default_bundle_preset()],
        },
        RepoConfig {
            repo_id: "triangle-rain-tr-engine".to_string(),
            label: "Triangle Rain (tr-engine)".to_string(),
            wsl_path: "/home/johnf/code/worktrees/tr-engine".to_string(),
            tab_id: "triangle-rain".to_string(),
            worktree_id: Some("tr-engine".to_string()),
            auto_stage: true,
            docs_globs: default_docs_globs(),
            code_globs: default_code_globs(),
            ignore_globs: default_ignore_globs(),
            bundle_presets: vec![default_bundle_preset()],
        },
        RepoConfig {
            repo_id: "intermediary".to_string(),
            label: "Intermediary".to_string(),
            wsl_path: "/home/johnf/code/intermediary".to_string(),
            tab_id: "intermediary".to_string(),
            worktree_id: None,
            auto_stage: true,
            docs_globs: default_docs_globs(),
            code_globs: default_code_globs(),
            ignore_globs: default_ignore_globs(),
            bundle_presets: vec![default_bundle_preset()],
        },
    ]
}

fn default_bundle_preset() -> BundlePreset {
    BundlePreset {
        preset_id: "context".to_string(),
        preset_name: "Context".to_string(),
        include_root: true,
        top_level_dirs: vec![],
    }
}

fn default_docs_globs() -> Vec<String> {
    vec![
        "docs/**".to_string(),
        "**/*.md".to_string(),
        "**/*.mdx".to_string(),
        "**/*.txt".to_string(),
        "**/*.rst".to_string(),
        "**/*.adoc".to_string(),
        "**/*.asciidoc".to_string(),
        "**/*.wiki".to_string(),
        "**/README*".to_string(),
    ]
}

fn default_code_globs() -> Vec<String> {
    vec![
        "src/**".to_string(),
        "app/**".to_string(),
        "agent/**".to_string(),
        "crates/**".to_string(),
        "src-tauri/**".to_string(),
        "**/*.ts".to_string(),
        "**/*.tsx".to_string(),
        "**/*.js".to_string(),
        "**/*.jsx".to_string(),
        "**/*.mjs".to_string(),
        "**/*.cjs".to_string(),
        "**/*.rs".to_string(),
        "**/*.toml".to_string(),
        "**/*.json".to_string(),
        "**/*.yaml".to_string(),
        "**/*.yml".to_string(),
        "**/*.py".to_string(),
        "**/*.go".to_string(),
    ]
}

fn default_ignore_globs() -> Vec<String> {
    vec![
        "**/node_modules/**".to_string(),
        "**/.git/**".to_string(),
        "**/dist/**".to_string(),
        "**/build/**".to_string(),
        "**/target/**".to_string(),
        "**/.cache/**".to_string(),
        "**/logs/**".to_string(),
    ]
}
