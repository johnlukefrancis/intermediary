// Path: src-tauri/src/lib/config/types/tests.rs
// Description: Tests for persisted configuration types

use super::CONFIG_VERSION;
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
