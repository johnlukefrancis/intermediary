// Path: src-tauri/src/lib/config/io.rs
// Description: Config file I/O with atomic writes and error handling

use crate::config::types::{PersistedConfig, CONFIG_VERSION};
use crate::paths::repo_root_resolver::{resolve_repo_root_from_input, RepoRootKind};
use serde_json::{json, Value};
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

    let mut migration_applied = migrate_legacy_repo_roots(&mut raw);
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

    config.config_version = CONFIG_VERSION;
    config
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
            let replacement = root_value
                .as_object()
                .and_then(|root_obj| root_obj.get("path").and_then(Value::as_str))
                .and_then(resolve_repo_root_from_input)
                .and_then(|resolved_root| {
                    let desired_kind = match resolved_root.kind {
                        RepoRootKind::Wsl => "wsl",
                        RepoRootKind::Windows => "windows",
                    };
                    let current_kind = root_value
                        .as_object()
                        .and_then(|root_obj| root_obj.get("kind"))
                        .and_then(Value::as_str);
                    let current_path = root_value
                        .as_object()
                        .and_then(|root_obj| root_obj.get("path"))
                        .and_then(Value::as_str);
                    if current_kind != Some(desired_kind)
                        || current_path != Some(resolved_root.path.as_str())
                    {
                        Some(match resolved_root.kind {
                            RepoRootKind::Wsl => {
                                json!({ "kind": "wsl", "path": resolved_root.path })
                            }
                            RepoRootKind::Windows => {
                                json!({ "kind": "windows", "path": resolved_root.path })
                            }
                        })
                    } else {
                        None
                    }
                });
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
        let Some(resolved_root) = resolve_repo_root_from_input(legacy_path) else {
            continue;
        };

        let root_json = match resolved_root.kind {
            RepoRootKind::Wsl => json!({ "kind": "wsl", "path": resolved_root.path }),
            RepoRootKind::Windows => json!({ "kind": "windows", "path": resolved_root.path }),
        };

        repo_obj.insert("root".to_string(), root_json);
        repo_obj.remove("wslPath");
        changed = true;
    }

    changed
}

#[cfg(test)]
#[path = "io/tests.rs"]
mod tests;
