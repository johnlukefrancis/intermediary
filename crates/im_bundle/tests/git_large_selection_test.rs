// Path: crates/im_bundle/tests/git_large_selection_test.rs
// Description: Windows-scale witness for host-safe selected Git diff path batching

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use im_bundle::plan::{BundlePlan, BundleSelection, GlobalExcludes};
use im_bundle::writer::write_bundle;
use serde_json::Value;
use tempfile::tempdir;

const FILE_COUNT: usize = 768;

#[test]
fn in_budget_selection_larger_than_windows_command_line_captures_complete_patch() {
    let root = tempdir().expect("tempdir");
    let repo = root.path().join("repo");
    std::fs::create_dir_all(repo.join("src/windows_argument_pressure")).expect("fixture directory");
    std::fs::create_dir_all(repo.join("src/private")).expect("private fixture directory");
    git(&repo, &["init", "-q"]);
    git(&repo, &["config", "user.email", "bundle@example.test"]);
    git(&repo, &["config", "user.name", "Bundle Test"]);

    let paths: Vec<_> = (0..FILE_COUNT).map(fixture_path).collect();
    let selected_path_bytes: usize = paths
        .iter()
        .map(|path| path.to_string_lossy().len() + 1)
        .sum();
    assert!(selected_path_bytes > 48 * 1024);
    assert!(selected_path_bytes < 256 * 1024);

    for path in &paths {
        write(&repo, path, b"baseline\n");
    }
    write(
        &repo,
        Path::new("src/private/hidden.txt"),
        b"private baseline\n",
    );
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-qm", "baseline"]);
    for (index, path) in paths.iter().enumerate() {
        write(&repo, path, format!("changed {index}\n").as_bytes());
    }
    write(
        &repo,
        Path::new("src/private/hidden.txt"),
        b"excluded large-selection sentinel\n",
    );

    let output = root.path().join("large-selection.zip");
    write_bundle(&plan(&repo, &output)).expect("large selected-path bundle");
    let mut archive = open_zip(&output);
    let manifest = read_json(&mut archive, "BUNDLE_MANIFEST.json");
    let patch = read_text(&mut archive, "BUNDLE_GIT_DIFF.patch");

    assert_eq!(manifest["git"]["status"], "complete");
    assert_eq!(manifest["git"]["counts"]["selectedChanged"], FILE_COUNT);
    assert_eq!(
        manifest["git"]["counts"]["selectedTrackedChanged"],
        FILE_COUNT
    );
    assert_eq!(manifest["git"]["issues"], serde_json::json!([]));
    assert_eq!(manifest["git"]["counts"]["omittedChangedPaths"], 1);
    assert_eq!(patch.matches("diff --git ").count(), FILE_COUNT);
    assert!(patch.contains(&paths[0].to_string_lossy().replace('\\', "/")));
    assert!(patch.contains(&paths[FILE_COUNT - 1].to_string_lossy().replace('\\', "/")));
    assert!(patch.contains(&format!("+changed {}", FILE_COUNT - 1)));
    assert!(!patch.contains("src/private/hidden.txt"));
    assert!(!patch.contains("excluded large-selection sentinel"));
    assert!(archive.by_name("src/private/hidden.txt").is_err());
}

fn fixture_path(index: usize) -> PathBuf {
    PathBuf::from(format!(
        "src/windows_argument_pressure/selected_file_{index:04}_with_a_deliberately_long_git_pathspec_name.txt"
    ))
}

fn plan(repo: &Path, output: &Path) -> BundlePlan {
    BundlePlan {
        output_path: output.to_path_buf(),
        repo_root: repo.to_path_buf(),
        repo_id: "large-selection".to_string(),
        preset_id: "context".to_string(),
        preset_name: "Context".to_string(),
        selection: BundleSelection {
            include_root: false,
            top_level_dirs: vec!["src".to_string()],
            included_subdirs: vec![],
            excluded_subdirs: vec!["src/private".to_string()],
            excluded_files: vec![],
        },
        built_at_iso: "2026-07-15T18:57:05Z".to_string(),
        global_excludes: GlobalExcludes::default(),
    }
}

fn git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(repo)
        .status()
        .expect("run git");
    assert!(status.success(), "git command failed: {args:?}");
}

fn write(repo: &Path, relative: &Path, contents: &[u8]) {
    std::fs::write(repo.join(relative), contents).expect("write fixture");
}

fn open_zip(path: &Path) -> zip::ZipArchive<std::fs::File> {
    zip::ZipArchive::new(std::fs::File::open(path).expect("open zip")).expect("read zip")
}

fn read_text(archive: &mut zip::ZipArchive<std::fs::File>, name: &str) -> String {
    String::from_utf8(read_bytes(archive, name)).expect("utf8 generated artifact")
}

fn read_bytes(archive: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Vec<u8> {
    let mut entry = archive.by_name(name).expect("zip entry");
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes).expect("read zip entry");
    bytes
}

fn read_json(archive: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Value {
    serde_json::from_slice(&read_bytes(archive, name)).expect("manifest json")
}
