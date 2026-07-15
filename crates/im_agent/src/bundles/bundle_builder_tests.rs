// Path: crates/im_agent/src/bundles/bundle_builder_tests.rs
// Description: Tests for bundle builder helpers

use std::io::Read;

use chrono::TimeZone;
use tempfile::TempDir;
use tokio::sync::mpsc;

use crate::protocol::{BundleSelection, GlobalExcludes};
use crate::staging::{PathBridgeConfig, StagingRootKind};

use super::bundle_builder_blocking::{
    build_bundle_blocking, cleanup_older_bundles, format_timestamp, BuildBundleBlockingOptions,
};

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
            included_subdirs: vec![],
            excluded_subdirs: vec![],
            excluded_files: vec![],
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
        progress_tx,
        im_bundle::cancel::BundleCancelToken::new(),
    ) {
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
            included_subdirs: vec![],
            excluded_subdirs: vec![],
            excluded_files: vec![],
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
        progress_tx,
        im_bundle::cancel::BundleCancelToken::new(),
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

#[test]
fn cancelled_build_keeps_last_good_bundle_and_removes_temp_output() {
    let root = TempDir::new().expect("tempdir");
    let repo_root = root.path().join("repo");
    let staging_root = root.path().join("staging");
    let bundle_dir = staging_root.join("bundles").join("repo").join("preset");
    std::fs::create_dir_all(&repo_root).expect("repo mkdir");
    std::fs::create_dir_all(&bundle_dir).expect("bundle mkdir");
    std::fs::write(repo_root.join("README.md"), "bundle content").expect("seed repo file");

    let last_good_path = bundle_dir.join("repo_preset_20240101_000000.zip");
    std::fs::write(&last_good_path, "good").expect("seed last good");

    let options = BuildBundleBlockingOptions {
        repo_id: "repo".to_string(),
        repo_root: repo_root.to_string_lossy().to_string(),
        preset_id: "preset".to_string(),
        preset_name: "Preset".to_string(),
        selection: BundleSelection {
            include_root: true,
            top_level_dirs: vec![],
            included_subdirs: vec![],
            excluded_subdirs: vec![],
            excluded_files: vec![],
        },
        staging: PathBridgeConfig {
            staging_host_root: staging_root.to_string_lossy().to_string(),
            staging_wsl_root: None,
        },
        staging_kind: StagingRootKind::Host,
        global_excludes: None,
    };
    let (progress_tx, _progress_rx) = mpsc::unbounded_channel();
    let cancel_token = im_bundle::cancel::BundleCancelToken::new();
    cancel_token.cancel();

    let err = match build_bundle_blocking(
        options,
        "2026-02-03T04:05:06Z".to_string(),
        "20260203_040506".to_string(),
        progress_tx,
        cancel_token,
    ) {
        Ok(_) => panic!("cancelled build should fail"),
        Err(err) => err,
    };

    assert_eq!(err.code(), "BUNDLE_BUILD_CANCELLED");
    assert!(last_good_path.exists());
    let temp_outputs = std::fs::read_dir(&bundle_dir)
        .expect("read bundle dir")
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
        .count();
    assert_eq!(temp_outputs, 0);
}

#[test]
fn explicit_global_excludes_without_build_include_scripts_build_files() {
    let root = TempDir::new().expect("tempdir");
    let repo_root = root.path().join("repo");
    let staging_root = root.path().join("staging");
    std::fs::create_dir_all(repo_root.join("Scripts/Build")).expect("scripts build mkdir");
    std::fs::create_dir_all(repo_root.join("app/node_modules")).expect("node_modules mkdir");
    std::fs::write(
        repo_root.join("Scripts/Build/Build-TriangleRainEditor.ps1"),
        "Write-Output 'build'\n",
    )
    .expect("seed build script");
    std::fs::write(repo_root.join("app/node_modules/noise.txt"), "ignored")
        .expect("seed ignored file");

    let options = BuildBundleBlockingOptions {
        repo_id: "repo".to_string(),
        repo_root: repo_root.to_string_lossy().to_string(),
        preset_id: "context".to_string(),
        preset_name: "Context".to_string(),
        selection: BundleSelection {
            include_root: false,
            top_level_dirs: vec!["Scripts".to_string(), "app".to_string()],
            included_subdirs: vec![],
            excluded_subdirs: vec![],
            excluded_files: vec![],
        },
        staging: PathBridgeConfig {
            staging_host_root: staging_root.to_string_lossy().to_string(),
            staging_wsl_root: None,
        },
        staging_kind: StagingRootKind::Host,
        global_excludes: Some(GlobalExcludes {
            dir_names: vec!["node_modules".to_string()],
            dir_suffixes: vec![],
            file_names: vec![],
            extensions: vec![],
            patterns: vec![],
        }),
    };
    let (progress_tx, _progress_rx) = mpsc::unbounded_channel();

    let result = build_bundle_blocking(
        options,
        "2026-05-21T22:13:17Z".to_string(),
        "20260521_221317".to_string(),
        progress_tx,
        im_bundle::cancel::BundleCancelToken::new(),
    )
    .expect("build should succeed");

    let file = std::fs::File::open(result.host_path).expect("open bundle");
    let mut archive = zip::ZipArchive::new(file).expect("open zip");
    assert!(archive
        .by_name("Scripts/Build/Build-TriangleRainEditor.ps1")
        .is_ok());
    assert!(archive.by_name("app/node_modules/noise.txt").is_err());

    let mut manifest = archive
        .by_name("BUNDLE_MANIFEST.json")
        .expect("manifest entry");
    let mut manifest_content = String::new();
    manifest
        .read_to_string(&mut manifest_content)
        .expect("read manifest");
    let manifest_json: serde_json::Value =
        serde_json::from_str(&manifest_content).expect("parse manifest");
    assert_eq!(
        manifest_json["effectiveGlobalExcludes"]["dirNames"],
        serde_json::json!(["node_modules"])
    );
    assert!(!manifest_json["effectiveGlobalExcludes"]["dirNames"]
        .as_array()
        .expect("dirNames array")
        .iter()
        .any(|value| value == "build"));
}
