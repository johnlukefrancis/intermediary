// Path: crates/im_agent/src/bundles/bundle_builder_tests.rs
// Description: Tests for bundle builder helpers

use chrono::TimeZone;
use tempfile::TempDir;

use super::bundle_builder_blocking::{cleanup_existing_bundles, format_timestamp};

#[test]
fn formats_timestamp_in_utc() {
    let date = chrono::Utc.with_ymd_and_hms(2026, 2, 3, 4, 5, 6).unwrap();
    let formatted = format_timestamp(date);
    assert_eq!(formatted, "20260203_040506");
}

#[test]
fn cleanup_existing_bundles_removes_matching_files() {
    let root = TempDir::new().expect("tempdir");
    let dir = root.path();
    std::fs::create_dir_all(dir).expect("mkdir");

    std::fs::write(dir.join("repo_preset_20240101_000000.zip"), "a").expect("write");
    std::fs::write(dir.join("repo_preset_20240102_000000.zip"), "b").expect("write");
    std::fs::write(dir.join("other_repo_preset_20240102_000000.zip"), "c").expect("write");

    cleanup_existing_bundles(dir, "repo", "preset");

    assert!(!dir.join("repo_preset_20240101_000000.zip").exists());
    assert!(!dir.join("repo_preset_20240102_000000.zip").exists());
    assert!(dir.join("other_repo_preset_20240102_000000.zip").exists());
}
