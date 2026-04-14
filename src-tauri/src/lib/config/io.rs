// Path: src-tauri/src/lib/config/io.rs
// Description: Config file I/O with atomic writes and error handling

use crate::config::generated_code_globs::GENERATED_CODE_EXTENSION_GLOBS;
use crate::config::types::{PersistedConfig, CONFIG_VERSION};
use crate::paths::repo_root_resolver::{resolve_legacy_repo_root_from_input, RepoRootKind};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::Path;

/// Errors that can occur during config operations
#[derive(Debug)]
pub enum ConfigError {
    /// Config file could not be read
    ReadFailed { source: std::io::Error },
    /// Config file contains invalid JSON
    ParseFailed { source: serde_json::Error },
    /// Config file could not be written
    WriteFailed { source: std::io::Error },
    /// Atomic rename failed
    RenameFailed { source: std::io::Error },
    /// Config version is from the future (newer than this app)
    FutureVersion { found: u32, max: u32 },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadFailed { source } => write!(f, "Failed to read config: {source}"),
            Self::ParseFailed { source } => write!(f, "Failed to parse config: {source}"),
            Self::WriteFailed { source } => write!(f, "Failed to write config: {source}"),
            Self::RenameFailed { source } => write!(f, "Failed to rename temp config: {source}"),
            Self::FutureVersion { found, max } => {
                write!(f, "Config version {found} is newer than supported ({max})")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

/// Load result indicating what happened during load
pub struct LoadResult {
    pub config: PersistedConfig,
    pub was_created: bool,
    pub migration_applied: bool,
}

/// Load config from disk, returning default if missing
pub fn load_from_disk(path: &Path) -> Result<LoadResult, ConfigError> {
    if !path.exists() {
        return Ok(LoadResult {
            config: PersistedConfig::default(),
            was_created: true,
            migration_applied: false,
        });
    }

    let contents = fs::read_to_string(path).map_err(|e| ConfigError::ReadFailed { source: e })?;
    let mut raw: Value =
        serde_json::from_str(&contents).map_err(|e| ConfigError::ParseFailed { source: e })?;

    let uses_legacy_compact_mode = raw.get("uiMode").and_then(Value::as_str) == Some("compact");
    let mut migration_applied = migrate_legacy_repo_roots(&mut raw);
    if uses_legacy_compact_mode {
        migration_applied = true;
    }
    let mut config: PersistedConfig =
        serde_json::from_value(raw).map_err(|e| ConfigError::ParseFailed { source: e })?;

    if config.config_version > CONFIG_VERSION {
        return Err(ConfigError::FutureVersion {
            found: config.config_version,
            max: CONFIG_VERSION,
        });
    }

    if config.config_version < CONFIG_VERSION {
        config = migrate_config(config);
        migration_applied = true;
    } else if migrate_compact_mode(&mut config) {
        migration_applied = true;
    }

    Ok(LoadResult {
        config,
        was_created: false,
        migration_applied,
    })
}
/// Save config to disk atomically (write temp, then rename)
pub fn save_to_disk(path: &Path, config: &PersistedConfig) -> Result<(), ConfigError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| ConfigError::WriteFailed { source: e })?;
    }

    let contents =
        serde_json::to_string_pretty(config).map_err(|e| ConfigError::ParseFailed { source: e })?;

    let temp_path = path.with_extension("json.tmp");
    let mut file =
        fs::File::create(&temp_path).map_err(|e| ConfigError::WriteFailed { source: e })?;
    file.write_all(contents.as_bytes())
        .map_err(|e| ConfigError::WriteFailed { source: e })?;
    file.sync_all()
        .map_err(|e| ConfigError::WriteFailed { source: e })?;
    drop(file);

    fs::rename(&temp_path, path).map_err(|e| ConfigError::RenameFailed { source: e })?;
    Ok(())
}
/// Apply migrations from older config versions
fn migrate_config(mut config: PersistedConfig) -> PersistedConfig {
    // Version 1 -> 2: Add excludedSubdirs to bundle selections.
    if config.config_version < 2 {
        for repo in config.bundle_selections.values_mut() {
            for selection in repo.values_mut() {
                if selection.excluded_subdirs.is_empty() {
                    selection.excluded_subdirs = Vec::new();
                }
            }
        }
    }

    // Version 2 -> 3: Remove tab/worktree identity fields.
    // Old lastActiveTabId values are handled by frontend fallback logic.

    // Version 11 -> 12: Normalize localhost agent host to loopback IP.
    if config.config_version < 12 && config.agent_host == "localhost" {
        config.agent_host = "127.0.0.1".to_string();
    }

    // Version 12 -> 13: Add agent auto-start + distro override fields.
    if config.config_version < 13
        && config
            .agent_distro
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
    {
        config.agent_distro = None;
    }

    // Version 15 -> 16: Replace repo.wslPath with path-native repo.root.
    // Structural conversion is handled in migrate_legacy_repo_roots().
    // Version 16 -> 17: Expand default codeGlobs coverage.
    if config.config_version < 17 {
        migrate_default_code_globs(&mut config);
    }
    // Version 17 -> 18: Rename repo root authority kind windows -> host.
    // Structural conversion is handled in migrate_legacy_repo_roots().
    // Version 18 -> 19: Add ui_mode (serde default handles missing field).
    // Version 19 -> 20: Add ui_state.window_bounds_by_mode (serde default handles missing field).
    // Version 20 -> 21: Remove compact ui_mode and fold compact bounds into standard.
    // Version 21 -> 22: Add window_opacity_percent (serde default handles missing field).
    // Version 22 -> 23: Add texture_intensity_percent (serde default handles missing field).
    // Version 23 -> 24: Remove legacy model-dir path excludes from the recommended baseline.
    if config.config_version < 21 {
        migrate_compact_mode(&mut config);
    }
    if config.config_version < 24 {
        migrate_legacy_model_dir_patterns(&mut config);
    }

    config.config_version = CONFIG_VERSION;
    config
}

fn migrate_compact_mode(config: &mut PersistedConfig) -> bool {
    let compact_bounds = config
        .ui_state
        .window_bounds_by_mode
        .get("compact")
        .copied();
    let mut changed = false;

    if let Some(bounds) = compact_bounds {
        if !config
            .ui_state
            .window_bounds_by_mode
            .contains_key("standard")
        {
            config
                .ui_state
                .window_bounds_by_mode
                .insert("standard".to_string(), bounds);
            changed = true;
        }
    }

    if config
        .ui_state
        .window_bounds_by_mode
        .remove("compact")
        .is_some()
    {
        changed = true;
    }

    changed
}

const CODE_ROOT_GLOBS: &[&str] = &["src/**", "app/**", "crates/**", "src-tauri/**"];
const INL_CODE_GLOB: &str = "**/*.inl";
const LEGACY_DEFAULT_CODE_GLOBS: &[&str] = &[
    "src/**",
    "app/**",
    "crates/**",
    "src-tauri/**",
    "**/*.ts",
    "**/*.tsx",
    "**/*.js",
    "**/*.jsx",
    "**/*.mjs",
    "**/*.cjs",
    "**/*.rs",
    "**/*.toml",
    "**/*.json",
    "**/*.yaml",
    "**/*.yml",
    "**/*.py",
    "**/*.go",
];

fn migrate_default_code_globs(config: &mut PersistedConfig) {
    for repo in config.repos.iter_mut() {
        if is_legacy_default_code_globs(&repo.code_globs) {
            repo.code_globs = default_code_globs();
        }
    }
}

fn is_legacy_default_code_globs(globs: &[String]) -> bool {
    let current = build_normalized_set(globs.iter().map(|value| value.as_str()));
    let legacy_minimal = build_normalized_set(LEGACY_DEFAULT_CODE_GLOBS.iter().copied());
    if current == legacy_minimal {
        return true;
    }

    let expanded = default_code_globs();
    let expanded_set = build_normalized_set(expanded.iter().map(|value| value.as_str()));
    if current == expanded_set {
        return true;
    }

    let expanded_without_inl = default_code_globs_without_inl();
    let expanded_without_inl_set =
        build_normalized_set(expanded_without_inl.iter().map(|value| value.as_str()));
    current == expanded_without_inl_set
}

fn default_code_globs() -> Vec<String> {
    let mut globs =
        Vec::with_capacity(CODE_ROOT_GLOBS.len() + GENERATED_CODE_EXTENSION_GLOBS.len());
    globs.extend(CODE_ROOT_GLOBS.iter().map(|value| value.to_string()));
    globs.extend(
        GENERATED_CODE_EXTENSION_GLOBS
            .iter()
            .map(|value| value.to_string()),
    );
    globs
}

fn default_code_globs_without_inl() -> Vec<String> {
    default_code_globs()
        .into_iter()
        .filter(|glob| !glob.eq_ignore_ascii_case(INL_CODE_GLOB))
        .collect()
}

const LEGACY_MODEL_DIR_PATTERNS: &[&str] = &["models", "weights", "checkpoints"];
const CURRENT_RECOMMENDED_PATTERNS: &[&str] = &[
    ".huggingface",
    "huggingface_hub",
    "wandb",
    "mlruns",
    "lightning_logs",
];

fn migrate_legacy_model_dir_patterns(config: &mut PersistedConfig) {
    let current_patterns = build_normalized_set(
        config
            .global_excludes
            .patterns
            .iter()
            .map(|value| value.as_str()),
    );
    let legacy_recommended_patterns = build_normalized_set(
        LEGACY_MODEL_DIR_PATTERNS
            .iter()
            .chain(CURRENT_RECOMMENDED_PATTERNS.iter())
            .copied(),
    );
    if current_patterns != legacy_recommended_patterns {
        return;
    }

    let legacy_model_dir_set = build_normalized_set(LEGACY_MODEL_DIR_PATTERNS.iter().copied());
    config.global_excludes.patterns.retain(|pattern| {
        !legacy_model_dir_set.contains(&pattern.trim().trim_matches('/').to_lowercase())
    });
}

fn build_normalized_set<'a>(values: impl Iterator<Item = &'a str>) -> HashSet<String> {
    values
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty())
        .collect()
}

fn migrate_legacy_repo_roots(raw: &mut Value) -> bool {
    let Some(config_obj) = raw.as_object_mut() else {
        return false;
    };
    let Some(repos_value) = config_obj.get_mut("repos") else {
        return false;
    };
    let Some(repos) = repos_value.as_array_mut() else {
        return false;
    };

    let mut changed = false;
    for repo in repos {
        let Some(repo_obj) = repo.as_object_mut() else {
            continue;
        };

        if let Some(root_value) = repo_obj.get_mut("root") {
            let current_kind = root_value
                .as_object()
                .and_then(|root_obj| root_obj.get("kind"))
                .and_then(Value::as_str);
            let current_path = root_value
                .as_object()
                .and_then(|root_obj| root_obj.get("path"))
                .and_then(Value::as_str);

            let replacement = match (current_kind, current_path) {
                (Some("wsl"), Some(path)) => {
                    resolve_legacy_repo_root_from_input(path).and_then(|resolved_root| {
                        let next_root = root_json_for_kind(resolved_root.kind, resolved_root.path);
                        if root_value != &next_root {
                            Some(next_root)
                        } else {
                            None
                        }
                    })
                }
                (Some("windows"), Some(path)) => {
                    let migrated_path = resolve_legacy_repo_root_from_input(path)
                        .filter(|resolved_root| resolved_root.kind == RepoRootKind::Host)
                        .map(|resolved_root| resolved_root.path)
                        .unwrap_or_else(|| path.trim().to_string());
                    let next_root = json!({ "kind": "host", "path": migrated_path });
                    if root_value != &next_root {
                        Some(next_root)
                    } else {
                        None
                    }
                }
                (Some("host"), Some(path)) => {
                    let trimmed = path.trim();
                    let next_root = json!({ "kind": "host", "path": trimmed });
                    if root_value != &next_root {
                        Some(next_root)
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some(new_root) = replacement {
                *root_value = new_root;
                changed = true;
            }
            if repo_obj.remove("wslPath").is_some() {
                changed = true;
            }
            continue;
        }

        let Some(legacy_path) = repo_obj.get("wslPath").and_then(Value::as_str) else {
            continue;
        };
        let Some(resolved_root) = resolve_legacy_repo_root_from_input(legacy_path) else {
            continue;
        };

        let root_json = root_json_for_kind(resolved_root.kind, resolved_root.path);

        repo_obj.insert("root".to_string(), root_json);
        repo_obj.remove("wslPath");
        changed = true;
    }

    changed
}

fn root_json_for_kind(kind: RepoRootKind, path: String) -> Value {
    match kind {
        RepoRootKind::Wsl => json!({ "kind": "wsl", "path": path }),
        RepoRootKind::Host => json!({ "kind": "host", "path": path }),
    }
}

#[cfg(test)]
#[path = "io/tests.rs"]
mod tests;
