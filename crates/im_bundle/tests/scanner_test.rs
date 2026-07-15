// Path: crates/im_bundle/tests/scanner_test.rs
// Description: Integration tests for bundle scanner behavior

use im_bundle::plan::{BundleSelection, GlobalExcludes};
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
            included_subdirs: vec![],
            excluded_subdirs: vec!["app/skip_me".to_string()],
            excluded_files: vec![],
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
fn scan_respects_nested_subdir_exclude() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();

    std::fs::create_dir_all(repo_root.join("app/src/components")).unwrap();
    std::fs::create_dir_all(repo_root.join("app/src/lib")).unwrap();
    std::fs::write(repo_root.join("app/src/components/secret.ts"), "skip").unwrap();
    std::fs::write(repo_root.join("app/src/lib/index.ts"), "keep").unwrap();

    let plan = BundlePlan {
        output_path: repo_root.join("out.zip"),
        repo_root: repo_root.to_path_buf(),
        repo_id: "repo".to_string(),
        preset_id: "full".to_string(),
        preset_name: "Full".to_string(),
        selection: BundleSelection {
            include_root: false,
            top_level_dirs: vec!["app".to_string()],
            included_subdirs: vec![],
            excluded_subdirs: vec!["app/src/components".to_string()],
            excluded_files: vec![],
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

    assert!(archive_paths.contains("app/src/lib/index.ts"));
    assert!(!archive_paths.contains("app/src/components/secret.ts"));
}

#[test]
fn scan_respects_excluded_files() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();

    std::fs::create_dir_all(repo_root.join("app/src")).unwrap();
    std::fs::write(repo_root.join("README.md"), "root").unwrap();
    std::fs::write(repo_root.join("keep.md"), "keep").unwrap();
    std::fs::write(repo_root.join("app/src/secret.ts"), "skip").unwrap();
    std::fs::write(repo_root.join("app/src/index.ts"), "keep").unwrap();

    let plan = BundlePlan {
        output_path: repo_root.join("out.zip"),
        repo_root: repo_root.to_path_buf(),
        repo_id: "repo".to_string(),
        preset_id: "full".to_string(),
        preset_name: "Full".to_string(),
        selection: BundleSelection {
            include_root: true,
            top_level_dirs: vec!["app".to_string()],
            included_subdirs: vec![],
            excluded_subdirs: vec![],
            excluded_files: vec!["README.md".to_string(), "app/src/secret.ts".to_string()],
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

    assert!(archive_paths.contains("keep.md"));
    assert!(archive_paths.contains("app/src/index.ts"));
    assert!(!archive_paths.contains("README.md"));
    assert!(!archive_paths.contains("app/src/secret.ts"));
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
            included_subdirs: vec![],
            excluded_subdirs: vec![],
            excluded_files: vec![],
        },
        built_at_iso: "2026-01-31T00:00:00Z".to_string(),
        global_excludes: GlobalExcludes::default(),
    };

    let mut progress = ProgressEmitter::new();
    let result = scan_bundle(&plan, &mut progress);
    assert!(matches!(
        result,
        Err(im_bundle::error::BundleError::InvalidPlan(_))
    ));
}

#[test]
fn reject_invalid_excluded_file() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    std::fs::create_dir(repo_root.join("app")).unwrap();

    let plan = BundlePlan {
        output_path: repo_root.join("out.zip"),
        repo_root: repo_root.to_path_buf(),
        repo_id: "repo".to_string(),
        preset_id: "full".to_string(),
        preset_name: "Full".to_string(),
        selection: BundleSelection {
            include_root: false,
            top_level_dirs: vec!["app".to_string()],
            included_subdirs: vec![],
            excluded_subdirs: vec![],
            excluded_files: vec!["../outside.txt".to_string()],
        },
        built_at_iso: "2026-01-31T00:00:00Z".to_string(),
        global_excludes: GlobalExcludes::default(),
    };

    let mut progress = ProgressEmitter::new();
    let result = scan_bundle(&plan, &mut progress);
    assert!(matches!(
        result,
        Err(im_bundle::error::BundleError::InvalidPlan(_))
    ));
}

#[test]
fn explicit_top_level_selection_overrides_default_directory_exclude() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();

    std::fs::create_dir(repo_root.join("app")).unwrap();
    std::fs::create_dir(repo_root.join("node_modules")).unwrap();
    std::fs::write(repo_root.join("app/index.ts"), "code").unwrap();
    std::fs::write(repo_root.join("node_modules/source.js"), "selected").unwrap();

    let plan = BundlePlan {
        output_path: repo_root.join("out.zip"),
        repo_root: repo_root.to_path_buf(),
        repo_id: "repo".to_string(),
        preset_id: "full".to_string(),
        preset_name: "Full".to_string(),
        selection: BundleSelection {
            include_root: false,
            top_level_dirs: vec!["app".to_string(), "node_modules".to_string()],
            included_subdirs: vec![],
            excluded_subdirs: vec![],
            excluded_files: vec![],
        },
        built_at_iso: "2026-01-31T00:00:00Z".to_string(),
        global_excludes: GlobalExcludes::default(),
    };

    let mut progress = ProgressEmitter::new();
    let result = scan_bundle(&plan, &mut progress).unwrap();
    assert_eq!(
        result.top_level_dirs_included,
        vec!["app".to_string(), "node_modules".to_string()]
    );
    assert!(result
        .entries
        .iter()
        .any(|entry| entry.archive_path == "node_modules/source.js"));
}

#[test]
fn explicitly_included_source_target_overrides_default_directory_exclude() {
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    std::fs::create_dir_all(repo_root.join("crates/wb_render_wgpu/src/target")).unwrap();
    std::fs::create_dir_all(repo_root.join("crates/other/target")).unwrap();
    std::fs::write(
        repo_root.join("crates/wb_render_wgpu/src/target/mod.rs"),
        "pub struct RenderTarget;\n",
    )
    .unwrap();
    std::fs::write(
        repo_root.join("crates/other/target/output.bin"),
        "generated",
    )
    .unwrap();

    let plan = BundlePlan {
        output_path: repo_root.join("out.zip"),
        repo_root: repo_root.to_path_buf(),
        repo_id: "repo".to_string(),
        preset_id: "full".to_string(),
        preset_name: "Full".to_string(),
        selection: BundleSelection {
            include_root: false,
            top_level_dirs: vec!["crates".to_string()],
            included_subdirs: vec!["crates/wb_render_wgpu/src/target".to_string()],
            excluded_subdirs: vec![],
            excluded_files: vec![],
        },
        built_at_iso: "2026-07-15T00:00:00Z".to_string(),
        global_excludes: GlobalExcludes {
            dir_names: vec!["target".to_string()],
            dir_suffixes: vec![],
            file_names: vec![],
            extensions: vec![],
            patterns: vec![],
        },
    };

    let mut progress = ProgressEmitter::new();
    let result = scan_bundle(&plan, &mut progress).unwrap();
    let archive_paths: Vec<_> = result
        .entries
        .iter()
        .map(|entry| entry.archive_path.as_str())
        .collect();

    assert!(archive_paths.contains(&"crates/wb_render_wgpu/src/target/mod.rs"));
    assert!(!archive_paths.contains(&"crates/other/target/output.bin"));
}
