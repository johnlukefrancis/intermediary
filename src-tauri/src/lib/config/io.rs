// Path: src-tauri/src/lib/config/io.rs
// Description: Config file I/O with atomic writes and error handling

use self::repo_root_migration::migrate_legacy_repo_roots;
use self::schema_migrations::{migrate_compact_mode, migrate_config};
use crate::config::types::{PersistedConfig, CONFIG_VERSION};
#[cfg(test)]
use serde_json::json;
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::Path;

#[path = "io/repo_root_migration.rs"]
mod repo_root_migration;
#[path = "io/schema_migrations.rs"]
mod schema_migrations;
#[cfg(test)]
use self::schema_migrations::{
    build_normalized_set, default_code_globs, default_code_globs_without_inl,
};

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
#[cfg(test)]
#[path = "io/migration_tests.rs"]
mod migration_tests;

#[cfg(test)]
#[path = "io/tests.rs"]
mod tests;
