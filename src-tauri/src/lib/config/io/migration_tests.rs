// Path: src-tauri/src/lib/config/io/migration_tests.rs
// Description: Focused config migration regression tests

use super::load_from_disk;
use crate::config::types::{PersistedConfig, CONFIG_VERSION};
use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn v24_bundle_selection_without_excluded_files_gets_default() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.json");

    let mut config_json = serde_json::to_value(PersistedConfig::default()).unwrap();
    config_json["configVersion"] = Value::Number(24_u64.into());
    config_json["bundleSelections"] = json!({
        "repo": {
            "context": {
                "includeRoot": true,
                "topLevelDirs": ["app"],
                "excludedSubdirs": ["app/generated"]
            }
        }
    });

    let mut file = fs::File::create(&path).unwrap();
    writeln!(file, "{config_json}").unwrap();

    let result = load_from_disk(&path).unwrap();
    assert!(result.migration_applied);
    assert_eq!(result.config.config_version, CONFIG_VERSION);

    let selection = result
        .config
        .bundle_selections
        .get("repo")
        .and_then(|presets| presets.get("context"))
        .expect("bundle selection should migrate");
    assert_eq!(
        selection.excluded_subdirs,
        vec!["app/generated".to_string()]
    );
    assert!(selection.excluded_files.is_empty());
}
