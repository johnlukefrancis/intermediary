// Path: src-tauri/src/lib/config/io/global_excludes_migrations.rs
// Description: Versioned migrations of the recommended global-exclude baseline

use super::schema_migrations::build_normalized_set;
use crate::config::types::PersistedConfig;
use im_bundle::global_excludes::RECOMMENDED_SCENE_FILE_SUFFIXES;

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

/// Version 27 -> 28: seed the recommended scene extensions into every config once.
/// The user never had these entries, so seeding overrides no decision; they land in
/// the explicit list, where removing them afterwards sticks.
pub(super) fn seed_recommended_scene_extensions(config: &mut PersistedConfig) {
    let current = build_normalized_set(
        config
            .global_excludes
            .extensions
            .iter()
            .map(|value| value.as_str()),
    );
    for suffix in RECOMMENDED_SCENE_FILE_SUFFIXES {
        if !current.contains(&suffix.to_lowercase()) {
            config.global_excludes.extensions.push(suffix.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trimmed_config_is_seeded_with_scene_extensions_only() {
        let mut config = PersistedConfig::default();
        config.global_excludes.extensions.clear();
        config.global_excludes.dir_names = vec!["target".to_string()];
        seed_recommended_scene_extensions(&mut config);
        let expected: Vec<String> = RECOMMENDED_SCENE_FILE_SUFFIXES
            .iter()
            .map(|value| value.to_string())
            .collect();
        assert_eq!(config.global_excludes.extensions, expected);
        assert_eq!(config.global_excludes.dir_names, vec!["target".to_string()]);
    }

    #[test]
    fn seeding_is_idempotent_and_case_insensitive() {
        let mut config = PersistedConfig::default();
        config.global_excludes.extensions = vec![".BLEND".to_string(), ".blend1".to_string()];
        seed_recommended_scene_extensions(&mut config);
        assert_eq!(
            config.global_excludes.extensions,
            vec![".BLEND".to_string(), ".blend1".to_string()]
        );
    }
}
