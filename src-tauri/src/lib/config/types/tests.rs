// Path: src-tauri/src/lib/config/types/tests.rs
// Description: Tests for persisted configuration types

use super::{PersistedConfig, CONFIG_VERSION};
use regex::Regex;

#[test]
fn ts_config_version_matches_rust() {
    let contents = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../app/src/shared/config/version.ts"
    ));
    let regex = Regex::new(r"CONFIG_VERSION\s*=\s*(\d+)").expect("valid regex");
    let caps = regex
        .captures(contents)
        .expect("CONFIG_VERSION not found in version.ts");
    let ts_version: u32 = caps[1]
        .parse()
        .expect("CONFIG_VERSION in version.ts must be a number");
    assert_eq!(
        ts_version, CONFIG_VERSION,
        "TS CONFIG_VERSION {ts_version} must match Rust CONFIG_VERSION {CONFIG_VERSION}"
    );
}

#[test]
fn demo_binding_roundtrips_and_older_configs_default_to_no_demo() {
    let mut config = PersistedConfig::default();
    let target = "http://127.0.0.1:8777/Docs/Reference/momo.html";
    config
        .demo_targets
        .insert("blorb".to_string(), target.to_string());
    let mut encoded = serde_json::to_value(config).unwrap();
    let restored: PersistedConfig = serde_json::from_value(encoded.clone()).unwrap();
    assert_eq!(
        restored.demo_targets.get("blorb").map(String::as_str),
        Some(target)
    );
    encoded.as_object_mut().unwrap().remove("demoTargets");
    let old: PersistedConfig = serde_json::from_value(encoded).unwrap();
    assert!(old.demo_targets.is_empty());
}
