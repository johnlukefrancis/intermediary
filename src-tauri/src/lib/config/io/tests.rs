// Path: src-tauri/src/lib/config/io/tests.rs
// Description: Unit tests for config I/O and migration behavior

use super::*;
use crate::config::types::{resolve_window_bounds_for_mode, RepoRoot, UiMode, UiWindowBounds};
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
    assert_eq!(result.config.window_opacity_percent, 100);
    assert_eq!(result.config.texture_intensity_percent, 100);
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

#[test]
fn test_unknown_ui_mode_coerces_to_standard() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.json");

    let mut config_json = serde_json::to_value(PersistedConfig::default()).unwrap();
    config_json["uiMode"] = Value::String("deckplus".to_string());

    let mut file = fs::File::create(&path).unwrap();
    writeln!(file, "{config_json}").unwrap();

    let result = load_from_disk(&path).unwrap();
    assert_eq!(result.config.ui_mode, UiMode::Standard);
}

#[test]
fn test_compact_ui_mode_coerces_to_standard() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.json");

    let mut config_json = serde_json::to_value(PersistedConfig::default()).unwrap();
    config_json["configVersion"] = Value::Number(20_u64.into());
    config_json["uiMode"] = Value::String("compact".to_string());

    let mut file = fs::File::create(&path).unwrap();
    writeln!(file, "{config_json}").unwrap();

    let result = load_from_disk(&path).unwrap();
    assert!(result.migration_applied);
    assert_eq!(result.config.ui_mode, UiMode::Standard);
}

#[test]
fn test_compact_window_bounds_migrate_to_standard_when_missing() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.json");

    let mut config_json = serde_json::to_value(PersistedConfig::default()).unwrap();
    config_json["configVersion"] = Value::Number(20_u64.into());
    config_json["uiState"]["windowBoundsByMode"] = json!({
        "compact": {
            "width": 1111,
            "height": 777
        }
    });

    let mut file = fs::File::create(&path).unwrap();
    writeln!(file, "{config_json}").unwrap();

    let result = load_from_disk(&path).unwrap();
    assert!(result.migration_applied);
    assert_eq!(result.config.ui_mode, UiMode::Standard);
    let standard_bounds = result
        .config
        .ui_state
        .window_bounds_by_mode
        .get("standard")
        .expect("standard bounds should be populated from legacy compact bounds");
    assert_eq!(standard_bounds.width, 1111);
    assert_eq!(standard_bounds.height, 777);
    assert!(!result
        .config
        .ui_state
        .window_bounds_by_mode
        .contains_key("compact"));
}

#[test]
fn test_v21_config_without_window_opacity_gets_default() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.json");

    let mut config_json = serde_json::to_value(PersistedConfig::default()).unwrap();
    config_json["configVersion"] = Value::Number(21_u64.into());
    if let Value::Object(map) = &mut config_json {
        map.remove("windowOpacityPercent");
    }

    let mut file = fs::File::create(&path).unwrap();
    writeln!(file, "{config_json}").unwrap();

    let result = load_from_disk(&path).unwrap();
    assert!(result.migration_applied);
    assert_eq!(result.config.config_version, CONFIG_VERSION);
    assert_eq!(result.config.window_opacity_percent, 100);
}

#[test]
fn test_v22_config_without_texture_intensity_gets_default() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.json");

    let mut config_json = serde_json::to_value(PersistedConfig::default()).unwrap();
    config_json["configVersion"] = Value::Number(22_u64.into());
    if let Value::Object(map) = &mut config_json {
        map.remove("textureIntensityPercent");
    }

    let mut file = fs::File::create(&path).unwrap();
    writeln!(file, "{config_json}").unwrap();

    let result = load_from_disk(&path).unwrap();
    assert!(result.migration_applied);
    assert_eq!(result.config.config_version, CONFIG_VERSION);
    assert_eq!(result.config.texture_intensity_percent, 100);
}

#[test]
fn test_v23_recommended_patterns_drop_legacy_model_dir_segments() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.json");

    let mut config_json = serde_json::to_value(PersistedConfig::default()).unwrap();
    config_json["configVersion"] = Value::Number(23_u64.into());
    config_json["globalExcludes"]["patterns"] = json!([
        "models",
        "weights",
        "checkpoints",
        ".huggingface",
        "huggingface_hub",
        "wandb",
        "mlruns",
        "lightning_logs"
    ]);

    let mut file = fs::File::create(&path).unwrap();
    writeln!(file, "{config_json}").unwrap();

    let result = load_from_disk(&path).unwrap();
    assert!(result.migration_applied);
    assert_eq!(result.config.config_version, CONFIG_VERSION);
    assert_eq!(
        result.config.global_excludes.patterns,
        vec![
            ".huggingface".to_string(),
            "huggingface_hub".to_string(),
            "wandb".to_string(),
            "mlruns".to_string(),
            "lightning_logs".to_string(),
        ]
    );
}

#[test]
fn test_unknown_window_bounds_mode_keys_are_ignored_at_runtime_resolution() {
    let mut config = PersistedConfig::default();
    config.ui_mode = UiMode::Handset;
    config.ui_state.window_bounds_by_mode.insert(
        "handset".to_string(),
        UiWindowBounds {
            width: 440,
            height: 680,
        },
    );
    config.ui_state.window_bounds_by_mode.insert(
        "experimental".to_string(),
        UiWindowBounds {
            width: 900,
            height: 900,
        },
    );

    let resolved = resolve_window_bounds_for_mode(&config, UiMode::Handset);
    assert_eq!(resolved.width, 440);
    assert_eq!(resolved.height, 680);
}

#[test]
fn test_migrates_legacy_wsl_path_to_host_root() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.json");

    let legacy = json!({
        "configVersion": 15,
        "agentHost": "127.0.0.1",
        "agentPort": 3141,
        "agentAutoStart": true,
        "agentDistro": null,
        "autoStageGlobal": true,
        "repos": [{
            "repoId": "repo",
            "label": "repo",
            "wslPath": "/mnt/c/code/repo",
            "autoStage": true,
            "docsGlobs": [],
            "codeGlobs": [],
            "ignoreGlobs": [],
            "bundlePresets": [{
                "presetId": "context",
                "presetName": "Context",
                "includeRoot": true,
                "topLevelDirs": []
            }]
        }],
        "recentFilesLimit": 200,
        "uiState": {
            "lastActiveTabId": null,
            "lastActiveGroupRepoIds": {}
        },
        "bundleSelections": {},
        "globalExcludes": {
            "dirNames": [],
            "dirSuffixes": [],
            "fileNames": [],
            "extensions": [],
            "patterns": []
        },
        "outputWindowsRoot": null,
        "tabThemes": {},
        "starredFiles": {},
        "themeMode": "dark"
    });

    let mut file = fs::File::create(&path).unwrap();
    writeln!(file, "{legacy}").unwrap();

    let result = load_from_disk(&path).unwrap();
    assert!(result.migration_applied);
    assert_eq!(result.config.config_version, CONFIG_VERSION);
    assert!(matches!(
        &result.config.repos[0].root,
        RepoRoot::Host { path } if path == r"C:\code\repo"
    ));
}

#[test]
fn test_migrates_legacy_default_code_globs_to_expanded_defaults() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.json");

    let legacy = json!({
        "configVersion": 16,
        "agentHost": "127.0.0.1",
        "agentPort": 3141,
        "agentAutoStart": true,
        "agentDistro": null,
        "autoStageGlobal": true,
        "repos": [{
            "repoId": "repo",
            "label": "repo",
            "root": { "kind": "wsl", "path": "/home/john/repo" },
            "autoStage": true,
            "docsGlobs": [],
            "codeGlobs": [
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
                "**/*.go"
            ],
            "ignoreGlobs": [],
            "bundlePresets": [{
                "presetId": "context",
                "presetName": "Context",
                "includeRoot": true,
                "topLevelDirs": []
            }]
        }],
        "recentFilesLimit": 200,
        "uiState": {
            "lastActiveTabId": null,
            "lastActiveGroupRepoIds": {}
        },
        "bundleSelections": {},
        "globalExcludes": {
            "dirNames": [],
            "dirSuffixes": [],
            "fileNames": [],
            "extensions": [],
            "patterns": []
        },
        "outputWindowsRoot": null,
        "tabThemes": {},
        "starredFiles": {},
        "themeMode": "dark"
    });

    let mut file = fs::File::create(&path).unwrap();
    writeln!(file, "{legacy}").unwrap();

    let result = load_from_disk(&path).unwrap();
    let code_globs = &result.config.repos[0].code_globs;
    assert!(code_globs.iter().any(|glob| glob == "**/*.cpp"));
    assert!(code_globs.iter().any(|glob| glob == "**/*.proto"));
}

#[test]
fn test_migrates_expanded_legacy_code_globs_without_inl() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.json");

    let expanded_without_inl = default_code_globs_without_inl();
    let legacy = json!({
        "configVersion": 16,
        "agentHost": "127.0.0.1",
        "agentPort": 3141,
        "agentAutoStart": true,
        "agentDistro": null,
        "autoStageGlobal": true,
        "repos": [{
            "repoId": "repo",
            "label": "repo",
            "root": { "kind": "wsl", "path": "/home/john/repo" },
            "autoStage": true,
            "docsGlobs": [],
            "codeGlobs": expanded_without_inl,
            "ignoreGlobs": [],
            "bundlePresets": [{
                "presetId": "context",
                "presetName": "Context",
                "includeRoot": true,
                "topLevelDirs": []
            }]
        }],
        "recentFilesLimit": 200,
        "uiState": {
            "lastActiveTabId": null,
            "lastActiveGroupRepoIds": {}
        },
        "bundleSelections": {},
        "globalExcludes": {
            "dirNames": [],
            "dirSuffixes": [],
            "fileNames": [],
            "extensions": [],
            "patterns": []
        },
        "outputWindowsRoot": null,
        "tabThemes": {},
        "starredFiles": {},
        "themeMode": "dark"
    });

    let mut file = fs::File::create(&path).unwrap();
    writeln!(file, "{legacy}").unwrap();

    let result = load_from_disk(&path).unwrap();
    let code_globs = &result.config.repos[0].code_globs;
    assert!(code_globs.iter().any(|glob| glob == "**/*.inl"));

    let migrated = build_normalized_set(code_globs.iter().map(|glob| glob.as_str()));
    let expected_globs = default_code_globs();
    let expected = build_normalized_set(expected_globs.iter().map(|glob| glob.as_str()));
    assert_eq!(migrated, expected);
}

#[test]
fn test_preserves_host_posix_root_kind_during_migration() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.json");

    let legacy = json!({
        "configVersion": 17,
        "agentHost": "127.0.0.1",
        "agentPort": 3141,
        "agentAutoStart": true,
        "agentDistro": null,
        "autoStageGlobal": true,
        "repos": [{
            "repoId": "repo",
            "label": "repo",
            "root": { "kind": "host", "path": "/Users/jl/code/repo" },
            "autoStage": true,
            "docsGlobs": [],
            "codeGlobs": [],
            "ignoreGlobs": [],
            "bundlePresets": [{
                "presetId": "context",
                "presetName": "Context",
                "includeRoot": true,
                "topLevelDirs": []
            }]
        }],
        "recentFilesLimit": 200,
        "uiState": {
            "lastActiveTabId": null,
            "lastActiveGroupRepoIds": {}
        },
        "bundleSelections": {},
        "globalExcludes": {
            "dirNames": [],
            "dirSuffixes": [],
            "fileNames": [],
            "extensions": [],
            "patterns": []
        },
        "outputWindowsRoot": null,
        "tabThemes": {},
        "starredFiles": {},
        "themeMode": "dark"
    });

    let mut file = fs::File::create(&path).unwrap();
    writeln!(file, "{legacy}").unwrap();

    let result = load_from_disk(&path).unwrap();
    assert!(matches!(
        &result.config.repos[0].root,
        RepoRoot::Host { path } if path == "/Users/jl/code/repo"
    ));
}

#[test]
fn test_preserves_host_mnt_root_kind_during_migration() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.json");

    let legacy = json!({
        "configVersion": 17,
        "agentHost": "127.0.0.1",
        "agentPort": 3141,
        "agentAutoStart": true,
        "agentDistro": null,
        "autoStageGlobal": true,
        "repos": [{
            "repoId": "repo",
            "label": "repo",
            "root": { "kind": "host", "path": "/mnt/d/work/repo" },
            "autoStage": true,
            "docsGlobs": [],
            "codeGlobs": [],
            "ignoreGlobs": [],
            "bundlePresets": [{
                "presetId": "context",
                "presetName": "Context",
                "includeRoot": true,
                "topLevelDirs": []
            }]
        }],
        "recentFilesLimit": 200,
        "uiState": {
            "lastActiveTabId": null,
            "lastActiveGroupRepoIds": {}
        },
        "bundleSelections": {},
        "globalExcludes": {
            "dirNames": [],
            "dirSuffixes": [],
            "fileNames": [],
            "extensions": [],
            "patterns": []
        },
        "outputWindowsRoot": null,
        "tabThemes": {},
        "starredFiles": {},
        "themeMode": "dark"
    });

    let mut file = fs::File::create(&path).unwrap();
    writeln!(file, "{legacy}").unwrap();

    let result = load_from_disk(&path).unwrap();
    assert!(matches!(
        &result.config.repos[0].root,
        RepoRoot::Host { path } if path == "/mnt/d/work/repo"
    ));
}
