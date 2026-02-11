// Path: crates/im_agent/src/bundles/bundle_builder_tests.rs
// Description: Tests for bundle builder helpers

use chrono::TimeZone;
use tempfile::TempDir;
use tokio::sync::mpsc;

use crate::protocol::BundleSelection;
use crate::staging::{PathBridgeConfig, StagingRootKind};

use super::bundle_builder_blocking::{
    build_bundle_blocking, cleanup_older_bundles, format_timestamp, BuildBundleBlockingOptions,
};
use super::git_info::GitInfo;

#[test]
fn formats_timestamp_in_utc() {
    let date = chrono::Utc.with_ymd_and_hms(2026, 2, 3, 4, 5, 6).unwrap();
    let formatted = format_timestamp(date);
    assert_eq!(formatted, "20260203_040506");
}

#[test]
fn cleanup_older_bundles_keeps_latest_file() {
    let root = TempDir::new().expect("tempdir");
    let dir = root.path();
    std::fs::create_dir_all(dir).expect("mkdir");

    let keep = dir.join("repo_preset_20240102_000000.zip");
    std::fs::write(dir.join("repo_preset_20240101_000000.zip"), "a").expect("write");
    std::fs::write(&keep, "b").expect("write");
    std::fs::write(dir.join("other_repo_preset_20240102_000000.zip"), "c").expect("write");

    cleanup_older_bundles(dir, "repo", "preset", &keep);

    assert!(!dir.join("repo_preset_20240101_000000.zip").exists());
    assert!(dir.join("repo_preset_20240102_000000.zip").exists());
    assert!(dir.join("other_repo_preset_20240102_000000.zip").exists());
}

#[test]
fn failed_build_keeps_last_good_bundle() {
    let root = TempDir::new().expect("tempdir");
    let repo_root = root.path().join("repo");
    let staging_root = root.path().join("staging");
    let bundle_dir = staging_root.join("bundles").join("repo").join("preset");
    std::fs::create_dir_all(&repo_root).expect("repo mkdir");
    std::fs::create_dir_all(&bundle_dir).expect("bundle mkdir");

    let last_good_path = bundle_dir.join("repo_preset_20240101_000000.zip");
    std::fs::write(&last_good_path, "good").expect("seed last good");

    let options = BuildBundleBlockingOptions {
        repo_id: "repo".to_string(),
        repo_root: repo_root.to_string_lossy().to_string(),
        preset_id: "preset".to_string(),
        preset_name: "Preset".to_string(),
        selection: BundleSelection {
            include_root: false,
            top_level_dirs: vec!["missing-dir".to_string()],
            excluded_subdirs: vec![],
        },
        staging: PathBridgeConfig {
            staging_host_root: staging_root.to_string_lossy().to_string(),
            staging_wsl_root: None,
        },
        staging_kind: StagingRootKind::Host,
        global_excludes: None,
    };
    let (progress_tx, _progress_rx) = mpsc::unbounded_channel();

    let err = match build_bundle_blocking(
        options,
        "2026-02-03T04:05:06Z".to_string(),
        "20260203_040506".to_string(),
        GitInfo {
            head_sha: None,
            short_sha: None,
            branch: None,
        },
        progress_tx,
    )
    {
        Ok(_) => panic!("build should fail with missing top-level directory"),
        Err(err) => err,
    };

    assert_eq!(err.code(), "BUNDLE_BUILD_FAILED");
    assert!(last_good_path.exists());
}

#[test]
fn successful_build_replaces_then_cleans_older_bundles() {
    let root = TempDir::new().expect("tempdir");
    let repo_root = root.path().join("repo");
    let staging_root = root.path().join("staging");
    let bundle_dir = staging_root.join("bundles").join("repo").join("preset");
    std::fs::create_dir_all(&repo_root).expect("repo mkdir");
    std::fs::create_dir_all(&bundle_dir).expect("bundle mkdir");
    std::fs::write(repo_root.join("README.md"), "bundle content").expect("seed repo file");

    let old_bundle = bundle_dir.join("repo_preset_20240101_000000.zip");
    std::fs::write(&old_bundle, "old").expect("seed old bundle");

    let options = BuildBundleBlockingOptions {
        repo_id: "repo".to_string(),
        repo_root: repo_root.to_string_lossy().to_string(),
        preset_id: "preset".to_string(),
        preset_name: "Preset".to_string(),
        selection: BundleSelection {
            include_root: true,
            top_level_dirs: vec![],
            excluded_subdirs: vec![],
        },
        staging: PathBridgeConfig {
            staging_host_root: staging_root.to_string_lossy().to_string(),
            staging_wsl_root: None,
        },
        staging_kind: StagingRootKind::Host,
        global_excludes: None,
    };
    let (progress_tx, _progress_rx) = mpsc::unbounded_channel();

    let result = build_bundle_blocking(
        options,
        "2026-02-03T04:05:06Z".to_string(),
        "20260203_040506".to_string(),
        GitInfo {
            head_sha: None,
            short_sha: Some("abc1234".to_string()),
            branch: None,
        },
        progress_tx,
    )
    .expect("build should succeed");

    let latest_path = std::path::PathBuf::from(result.host_path);
    assert!(latest_path.exists());
    assert!(!old_bundle.exists());

    let matching = std::fs::read_dir(&bundle_dir)
        .expect("read bundle dir")
        .flatten()
        .filter(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            name.starts_with("repo_preset_") && name.ends_with(".zip")
        })
        .count();
    assert_eq!(matching, 1);
}
