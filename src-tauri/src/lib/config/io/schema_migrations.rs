// Path: src-tauri/src/lib/config/io/schema_migrations.rs
// Description: Versioned persisted-config schema migrations

use crate::config::generated_code_globs::GENERATED_CODE_EXTENSION_GLOBS;
use crate::config::types::{PersistedConfig, CONFIG_VERSION};
use std::collections::HashSet;

pub(super) fn migrate_config(mut config: PersistedConfig) -> PersistedConfig {
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
    // Version 24 -> 25: Add bundle selection excluded_files
    // (serde default handles missing field).
    // Version 25 -> 26: Raise the default recent file retention.
    if config.config_version < 26 && config.recent_files_limit == 40 {
        config.recent_files_limit = 200;
    }

    config.config_version = CONFIG_VERSION;
    config
}

pub(super) fn migrate_compact_mode(config: &mut PersistedConfig) -> bool {
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

pub(super) fn default_code_globs() -> Vec<String> {
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

pub(super) fn default_code_globs_without_inl() -> Vec<String> {
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

pub(super) fn build_normalized_set<'a>(values: impl Iterator<Item = &'a str>) -> HashSet<String> {
    values
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty())
        .collect()
}
