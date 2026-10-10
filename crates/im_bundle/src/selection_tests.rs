// Path: crates/im_bundle/src/selection_tests.rs
// Description: Canonical selection predicate regression coverage for current and missing paths

use super::*;

fn selector() -> BundleSelector {
    BundleSelector::new(
        &BundleSelection {
            include_root: true,
            top_level_dirs: vec!["src".to_string()],
            included_subdirs: vec![],
            included_files: vec![],
            excluded_subdirs: vec!["src/private".to_string()],
            excluded_files: vec!["src/skip.rs".to_string()],
        },
        &GlobalExcludes {
            dir_names: vec!["target".to_string()],
            dir_suffixes: vec![],
            file_names: vec![],
            extensions: vec![".bin".to_string()],
            patterns: vec![],
        },
    )
    .expect("selector")
}

#[test]
fn applies_one_predicate_to_current_and_missing_paths() {
    let selector = selector();
    assert!(selector.admits_file(Path::new("README.md")));
    assert!(selector.admits_file(Path::new("src/main.rs")));
    assert!(selector.admits_file(Path::new("src/deleted.rs")));
    assert!(!selector.admits_file(Path::new("src/private/key.rs")));
    assert!(!selector.admits_file(Path::new("src/skip.rs")));
    assert!(!selector.admits_file(Path::new("src/blob.bin")));
    assert!(!selector.admits_file(Path::new("other/file.rs")));
}

#[test]
fn explicit_top_level_selection_overrides_directory_name_exclude() {
    let selector = selector();
    assert!(!selector.admits_file(Path::new("target/output.rs")));

    let selector = BundleSelector::new(
        &BundleSelection {
            include_root: false,
            top_level_dirs: vec!["target".to_string()],
            included_subdirs: vec![],
            included_files: vec![],
            excluded_subdirs: vec![],
            excluded_files: vec![],
        },
        &GlobalExcludes {
            dir_names: vec!["target".to_string()],
            dir_suffixes: vec![],
            file_names: vec![],
            extensions: vec![],
            patterns: vec![],
        },
    )
    .expect("selector");

    assert!(selector.admits_directory(Path::new("target")));
    assert!(selector.admits_file(Path::new("target/output.rs")));
}

#[test]
fn explicit_nested_selection_overrides_only_its_directory_name_exclude() {
    let selector = BundleSelector::new(
        &BundleSelection {
            include_root: false,
            top_level_dirs: vec!["crates".to_string()],
            included_subdirs: vec!["crates/wb_render_wgpu/src/target".to_string()],
            included_files: vec![],
            excluded_subdirs: vec![],
            excluded_files: vec![],
        },
        &GlobalExcludes {
            dir_names: vec!["target".to_string(), "node_modules".to_string()],
            dir_suffixes: vec![],
            file_names: vec![],
            extensions: vec![],
            patterns: vec![],
        },
    )
    .expect("selector");

    assert!(selector.admits_file(Path::new("crates/wb_render_wgpu/src/target/mod.rs")));
    assert!(!selector.admits_file(Path::new(
        "crates/wb_render_wgpu/src/target/node_modules/noise.js"
    )));
    assert!(!selector.admits_file(Path::new("crates/other/target/output.rs")));
}
