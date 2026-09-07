// Path: src-tauri/src/lib/config/io/global_excludes_migrations.rs
// Description: Versioned migrations of the recommended global-exclude baseline

use super::schema_migrations::build_normalized_set;
use crate::config::types::PersistedConfig;
use im_bundle::global_excludes::{recommended_global_excludes, RECOMMENDED_SCENE_FILE_SUFFIXES};

const LEGACY_MODEL_DIR_PATTERNS: &[&str] = &["models", "weights", "checkpoints"];
const CURRENT_RECOMMENDED_PATTERNS: &[&str] = &[
    ".huggingface",
    "huggingface_hub",
    "wandb",
    "mlruns",
    "lightning_logs",
];

pub(super) fn migrate_legacy_model_dir_patterns(config: &mut PersistedConfig) {
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

/// Version 26 -> 27: a config that still carries every recommended entry as it
/// stood before the scene extensions were added is the recommended baseline and
/// gains them. A list the user has trimmed is left as written.
pub(super) fn migrate_recommended_scene_extensions(config: &mut PersistedConfig) {
    let recommended = recommended_global_excludes();
    let scene = build_normalized_set(RECOMMENDED_SCENE_FILE_SUFFIXES.iter().copied());
    let excludes = &config.global_excludes;

    let carries_all = |current: &[String], expected: &[String]| {
        let current = build_normalized_set(current.iter().map(|value| value.as_str()));
        expected
            .iter()
            .map(|value| value.trim().to_lowercase())
            .filter(|value| !value.is_empty() && !scene.contains(value))
            .all(|value| current.contains(&value))
    };

    let carries_baseline = carries_all(&excludes.extensions, &recommended.extensions)
        && carries_all(&excludes.dir_names, &recommended.dir_names)
        && carries_all(&excludes.dir_suffixes, &recommended.dir_suffixes)
        && carries_all(&excludes.file_names, &recommended.file_names)
        && carries_all(&excludes.patterns, &recommended.patterns);
    if !carries_baseline {
        return;
    }

    let current = build_normalized_set(excludes.extensions.iter().map(|value| value.as_str()));
    for suffix in RECOMMENDED_SCENE_FILE_SUFFIXES {
        if !current.contains(&suffix.to_lowercase()) {
            config.global_excludes.extensions.push(suffix.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline_without_scene() -> PersistedConfig {
        let mut config = PersistedConfig::default();
        config
            .global_excludes
            .extensions
            .retain(|value| !RECOMMENDED_SCENE_FILE_SUFFIXES.contains(&value.as_str()));
        config
    }

    #[test]
    fn baseline_config_gains_scene_extensions() {
        let mut config = baseline_without_scene();
        migrate_recommended_scene_extensions(&mut config);
        for suffix in RECOMMENDED_SCENE_FILE_SUFFIXES {
            assert!(config.global_excludes.extensions.iter().any(|v| v == suffix));
        }
    }

    #[test]
    fn trimmed_config_is_left_as_written() {
        let mut config = baseline_without_scene();
        config.global_excludes.extensions.retain(|value| value != ".bin");
        let before = config.global_excludes.extensions.clone();
        migrate_recommended_scene_extensions(&mut config);
        assert_eq!(config.global_excludes.extensions, before);
    }

    #[test]
    fn migration_is_idempotent() {
        let mut config = PersistedConfig::default();
        let before = config.global_excludes.extensions.clone();
        migrate_recommended_scene_extensions(&mut config);
        assert_eq!(config.global_excludes.extensions, before);
    }
}
