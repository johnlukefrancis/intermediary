// Path: src-tauri/src/lib/config/io.rs
// Description: Config file I/O with atomic writes and error handling

use crate::config::types::{PersistedConfig, CONFIG_VERSION};
use crate::paths::wsl_convert::windows_to_wsl_path;
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
    // If file doesn't exist, return defaults
    if !path.exists() {
        return Ok(LoadResult {
            config: PersistedConfig::default(),
            was_created: true,
            migration_applied: false,
        });
    }

    // Read and parse
    let contents = fs::read_to_string(path).map_err(|e| ConfigError::ReadFailed { source: e })?;

    let mut config: PersistedConfig =
        serde_json::from_str(&contents).map_err(|e| ConfigError::ParseFailed { source: e })?;

    // Check for future version
    if config.config_version > CONFIG_VERSION {
        return Err(ConfigError::FutureVersion {
            found: config.config_version,
            max: CONFIG_VERSION,
        });
    }

    // Apply migrations if needed
    let mut migration_applied = config.config_version < CONFIG_VERSION;
    if migration_applied {
        config = migrate_config(config);
    }

    // Normalize repo paths to WSL form (accept Windows/UNC inputs).
    if normalize_repo_paths(&mut config) {
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
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| ConfigError::WriteFailed { source: e })?;
    }

    // Serialize with pretty printing for human readability
    let contents =
        serde_json::to_string_pretty(config).map_err(|e| ConfigError::ParseFailed { source: e })?;

    // Write to temp file first
    let temp_path = path.with_extension("json.tmp");
    let mut file =
        fs::File::create(&temp_path).map_err(|e| ConfigError::WriteFailed { source: e })?;
    file.write_all(contents.as_bytes())
        .map_err(|e| ConfigError::WriteFailed { source: e })?;
    file.sync_all()
        .map_err(|e| ConfigError::WriteFailed { source: e })?;
    drop(file);

    // Atomic rename
    fs::rename(&temp_path, path).map_err(|e| ConfigError::RenameFailed { source: e })?;

    Ok(())
}

/// Apply migrations from older config versions
fn migrate_config(mut config: PersistedConfig) -> PersistedConfig {
    // Version 1 -> 2: Add excludedSubdirs to bundle selections
    if config.config_version < 2 {
        for repo in config.bundle_selections.values_mut() {
            for selection in repo.values_mut() {
                if selection.excluded_subdirs.is_empty() {
                    selection.excluded_subdirs = Vec::new();
                }
            }
        }
    }

    // Version 2 -> 3: Remove tabId, worktreeId, lastTriangleRainWorktreeId
    // These fields are simply ignored during deserialization (serde ignores unknown fields).
    // Old lastActiveTabId values may not match repoIds; app.tsx handles fallback.

    // Version 11 -> 12: Normalize localhost agent host to loopback IP.
    if config.config_version < 12 && config.agent_host == "localhost" {
        config.agent_host = "127.0.0.1".to_string();
    }

    // Version 12 -> 13: Add agent auto-start + distro override fields.
    if config.config_version < 13 {
        if config.agent_distro.as_deref().unwrap_or("").trim().is_empty() {
            config.agent_distro = None;
        }
        // Ensure auto-start is defaulted when missing.
        if config.agent_auto_start == false {
            // Keep explicit false; defaults handled by serde for missing fields.
        }
    }

    // Update version to current
    config.config_version = CONFIG_VERSION;
    config
}

fn normalize_repo_paths(config: &mut PersistedConfig) -> bool {
    let mut changed = false;
    for repo in &mut config.repos {
        if let Some(normalized) = windows_to_wsl_path(&repo.wsl_path) {
            if normalized != repo.wsl_path {
                repo.wsl_path = normalized;
                changed = true;
            }
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_load_missing_returns_default() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");

        let result = load_from_disk(&path).unwrap();
        assert!(result.was_created);
        assert!(!result.migration_applied);
        assert_eq!(result.config.agent_port, 3141);
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");

        let mut config = PersistedConfig::default();
        config.agent_port = 9999;

        save_to_disk(&path, &config).unwrap();
        let result = load_from_disk(&path).unwrap();

        assert!(!result.was_created);
        assert_eq!(result.config.agent_port, 9999);
    }

    #[test]
    fn test_atomic_write_creates_no_temp_on_success() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        let temp_path = path.with_extension("json.tmp");

        save_to_disk(&path, &PersistedConfig::default()).unwrap();

        assert!(path.exists());
        assert!(!temp_path.exists());
    }

    #[test]
    fn test_future_version_rejected() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");

        let mut config = PersistedConfig::default();
        config.config_version = CONFIG_VERSION + 1;

        let mut file = fs::File::create(&path).unwrap();
        let payload = serde_json::to_string(&config).unwrap();
        writeln!(file, "{payload}").unwrap();

        let result = load_from_disk(&path);
        assert!(matches!(result, Err(ConfigError::FutureVersion { .. })));
    }
}
