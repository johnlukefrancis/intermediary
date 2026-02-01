// Path: crates/im_bundle/tests/scanner_test.rs
// Description: Integration tests for bundle scanner behavior

use im_bundle::plan::{BundleGitInfo, BundleSelection, GlobalExcludes};
use im_bundle::progress::ProgressEmitter;
use im_bundle::scanner::scan_bundle;
use im_bundle::BundlePlan;
use tempfile::tempdir;

#[test]
fn scan_respects_ignore_and_exclude() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();

    std::fs::create_dir(repo_root.join("app")).unwrap();
    std::fs::create_dir(repo_root.join("node_modules")).unwrap();
    std::fs::create_dir_all(repo_root.join("app/skip_me")).unwrap();

    std::fs::write(repo_root.join("README.md"), "root").unwrap();
    std::fs::write(repo_root.join(".env"), "secret").unwrap();
    std::fs::write(repo_root.join("app/index.ts"), "code").unwrap();
    std::fs::write(repo_root.join("app/skip_me/secret.txt"), "skip").unwrap();

    let plan = BundlePlan {
        output_path: repo_root.join("out.zip"),
        repo_root: repo_root.to_path_buf(),
        repo_id: "repo".to_string(),
        preset_id: "full".to_string(),
        preset_name: "Full".to_string(),
        selection: BundleSelection {
            include_root: true,
            top_level_dirs: vec!["app".to_string()],
            excluded_subdirs: vec!["app/skip_me".to_string()],
        },
        git: BundleGitInfo {
            head_sha: None,
            short_sha: None,
            branch: None,
        },
        built_at_iso: "2026-01-31T00:00:00Z".to_string(),
        global_excludes: GlobalExcludes::default(),
    };

    let mut progress = ProgressEmitter::new();
    let result = scan_bundle(&plan, &mut progress).unwrap();

    let archive_paths: std::collections::HashSet<_> = result
        .entries
        .iter()
        .map(|entry| entry.archive_path.as_str())
        .collect();

    assert!(archive_paths.contains("README.md"));
    assert!(archive_paths.contains("app/index.ts"));
    assert!(!archive_paths.contains(".env"));
    assert!(!archive_paths.contains("app/skip_me/secret.txt"));
}

#[test]
fn reject_invalid_top_level_dir() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();

    let plan = BundlePlan {
        output_path: repo_root.join("out.zip"),
        repo_root: repo_root.to_path_buf(),
        repo_id: "repo".to_string(),
        preset_id: "full".to_string(),
        preset_name: "Full".to_string(),
        selection: BundleSelection {
            include_root: false,
            top_level_dirs: vec!["../escape".to_string()],
            excluded_subdirs: vec![],
        },
        git: BundleGitInfo {
            head_sha: None,
            short_sha: None,
            branch: None,
        },
        built_at_iso: "2026-01-31T00:00:00Z".to_string(),
        global_excludes: GlobalExcludes::default(),
    };

    let mut progress = ProgressEmitter::new();
    let result = scan_bundle(&plan, &mut progress);
    assert!(matches!(result, Err(im_bundle::error::BundleError::InvalidPlan(_))));
}

#[test]
fn ignores_top_level_ignored_dirs_without_error() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();

    std::fs::create_dir(repo_root.join("app")).unwrap();
    std::fs::create_dir(repo_root.join("node_modules")).unwrap();
    std::fs::write(repo_root.join("app/index.ts"), "code").unwrap();

    let plan = BundlePlan {
        output_path: repo_root.join("out.zip"),
        repo_root: repo_root.to_path_buf(),
        repo_id: "repo".to_string(),
        preset_id: "full".to_string(),
        preset_name: "Full".to_string(),
        selection: BundleSelection {
            include_root: false,
            top_level_dirs: vec!["app".to_string(), "node_modules".to_string()],
            excluded_subdirs: vec![],
        },
        git: BundleGitInfo {
            head_sha: None,
            short_sha: None,
            branch: None,
        },
        built_at_iso: "2026-01-31T00:00:00Z".to_string(),
        global_excludes: GlobalExcludes::default(),
    };

    let mut progress = ProgressEmitter::new();
    let result = scan_bundle(&plan, &mut progress).unwrap();
    assert_eq!(result.top_level_dirs_included, vec!["app".to_string()]);
}
