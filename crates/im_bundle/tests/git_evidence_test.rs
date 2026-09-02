// Path: crates/im_bundle/tests/git_evidence_test.rs
// Description: End-to-end witness tests for selection-bounded bundle Git evidence

use std::io::Read;
use std::path::Path;
use std::process::Command;

use im_bundle::plan::{BundlePlan, BundleSelection, GlobalExcludes};
use im_bundle::writer::write_bundle;
use serde_json::Value;
use tempfile::tempdir;

#[test]
fn dirty_repo_bundle_carries_exact_selected_head_evidence() {
    let root = tempdir().expect("tempdir");
    let repo = root.path().join("repo");
    std::fs::create_dir_all(repo.join("src/private")).expect("repo files");
    std::fs::create_dir_all(repo.join("src/cache")).expect("repo cache");
    git(&repo, &["init", "-q"]);
    git(&repo, &["config", "user.email", "bundle@example.test"]);
    git(&repo, &["config", "user.name", "Bundle Test"]);

    write(&repo, "src/staged.txt", b"staged base\n");
    write(&repo, "src/unstaged.txt", b"unstaged base\n");
    write(&repo, "src/deleted.txt", b"delete me\n");
    write(&repo, "src/rename_old.txt", b"rename content\n");
    write(&repo, "src/excluded.txt", b"private base\n");
    write(&repo, "src/private/hidden.txt", b"subdir private base\n");
    write(&repo, "src/cache/hidden.txt", b"global private base\n");
    write(&repo, "src/blob.dat", b"\0binary base\0");
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-qm", "baseline"]);

    write(&repo, "src/staged.txt", b"staged intermediate\n");
    git(&repo, &["add", "src/staged.txt"]);
    write(&repo, "src/staged.txt", b"staged plus final worktree\n");
    write(&repo, "src/unstaged.txt", b"unstaged final\n");
    write(&repo, "src/untracked.txt", b"new selected file\n");
    std::fs::remove_file(repo.join("src/deleted.txt")).expect("delete tracked file");
    git(&repo, &["mv", "src/rename_old.txt", "src/rename_new.txt"]);
    write(&repo, "src/excluded.txt", b"private changed\n");
    write(&repo, "src/private/hidden.txt", b"subdir private changed\n");
    write(&repo, "src/cache/hidden.txt", b"global private changed\n");
    write(&repo, "src/blob.dat", b"\0binary changed payload\0");

    let output = root.path().join("dirty.zip");
    let result = write_bundle(&plan(&repo, &output)).expect("write dirty bundle");
    let mut archive = open_zip(&output);
    let manifest = read_json(&mut archive, "BUNDLE_MANIFEST.json");
    let status = read_text(&mut archive, "BUNDLE_GIT_STATUS.txt");
    let patch = read_text(&mut archive, "BUNDLE_GIT_DIFF.patch");
    let handoff = read_text(&mut archive, "BUNDLE_HANDOFF.md");
    let index_patch = read_text(&mut archive, "BUNDLE_GIT_INDEX_DIFF.patch");
    let worktree_patch = read_text(&mut archive, "BUNDLE_GIT_WORKTREE_DIFF.patch");
    let omitted = read_text(&mut archive, "BUNDLE_GIT_OMITTED_PATHS.txt");

    assert_eq!(manifest["bundleFormatVersion"], 3);
    assert_eq!(manifest["git"]["contractVersion"], 2);
    assert_eq!(manifest["git"]["comparisonBase"], "HEAD");
    assert_eq!(manifest["git"]["status"], "complete");
    assert_eq!(manifest["git"]["patchDeletions"], "full");
    assert!(manifest["git"]["capturedAt"].as_str().is_some());
    assert!(manifest["git"]["headSha"]
        .as_str()
        .is_some_and(|sha| matches!(sha.len(), 40 | 64)));
    assert!(manifest["git"]["branch"].as_str().is_some());
    assert_eq!(
        manifest["git"]["artifacts"]["status"],
        "BUNDLE_GIT_STATUS.txt"
    );
    assert_eq!(
        manifest["git"]["artifacts"]["diff"],
        "BUNDLE_GIT_DIFF.patch"
    );
    assert_eq!(
        manifest["git"]["artifacts"]["indexDiff"],
        "BUNDLE_GIT_INDEX_DIFF.patch"
    );
    assert_eq!(
        manifest["git"]["artifacts"]["worktreeDiff"],
        "BUNDLE_GIT_WORKTREE_DIFF.patch"
    );
    assert_eq!(
        manifest["git"]["artifacts"]["omittedPaths"],
        "BUNDLE_GIT_OMITTED_PATHS.txt"
    );
    assert_eq!(manifest["git"]["artifacts"]["handoff"], "BUNDLE_HANDOFF.md");
    assert_eq!(
        manifest["git"]["candidateIndexTreeSha"],
        git_stdout(&repo, &["write-tree"])
    );
    assert_eq!(
        manifest["git"]["incompleteArtifacts"],
        serde_json::json!([])
    );
    assert_eq!(manifest["git"]["issues"], serde_json::json!([]));
    assert_eq!(manifest["git"]["repoDirty"], true);
    assert_eq!(manifest["git"]["selectionDirty"], true);
    assert_eq!(manifest["git"]["counts"]["selectedUntracked"], 1);
    assert_eq!(manifest["git"]["counts"]["selectedChanged"], 7);
    assert_eq!(manifest["git"]["counts"]["selectedTrackedChanged"], 6);
    assert_eq!(manifest["git"]["counts"]["selectedDeleted"], 1);
    assert_eq!(manifest["git"]["counts"]["selectedRenamed"], 1);
    assert_eq!(manifest["git"]["counts"]["omittedChangedPaths"], 3);
    assert_eq!(manifest["fileCount"], result.file_count);
    assert_eq!(archive.len() as u64, result.file_count);

    assert!(status.contains("src/staged.txt"));
    assert!(status.contains("src/unstaged.txt"));
    assert!(status.contains("src/untracked.txt [untracked; no HEAD ancestor]"));
    assert!(status.contains("src/deleted.txt"));
    assert!(status.contains("src/rename_old.txt -> src/rename_new.txt"));
    assert!(status.contains("Selected diff stat"));
    assert!(status.contains("Selected diff name-status"));
    assert!(status.contains("M\tsrc/staged.txt"));
    assert!(!status.contains("diff --git "));
    assert!(!status.contains("@@ -"));
    assert!(status.contains("additional changed paths excluded"));
    assert!(status.contains("BUNDLE_GIT_OMITTED_PATHS.txt"));
    assert!(!status.contains("src/excluded.txt"));
    assert!(status.contains("Candidate index tree: "));

    // Staged versus unstaged boundary: index patch is HEAD -> index, worktree
    // patch is index -> working tree, and neither leaks excluded names.
    assert!(index_patch.contains("+staged intermediate"));
    assert!(!index_patch.contains("staged plus final worktree"));
    assert!(!index_patch.contains("unstaged final"));
    assert!(index_patch.contains("rename from src/rename_old.txt"));
    assert!(worktree_patch.contains("-staged intermediate"));
    assert!(worktree_patch.contains("+staged plus final worktree"));
    assert!(worktree_patch.contains("+unstaged final"));
    assert!(worktree_patch.contains("deleted file mode"));
    assert!(!worktree_patch.contains("src/untracked.txt"));
    for excluded in [
        "src/excluded.txt",
        "src/private/hidden.txt",
        "src/cache/hidden.txt",
    ] {
        assert!(
            !index_patch.contains(excluded),
            "{excluded} leaked into index patch"
        );
        assert!(
            !worktree_patch.contains(excluded),
            "{excluded} leaked into worktree patch"
        );
    }

    // Omitted changed paths are named with their reason; their content is not bundled.
    assert!(omitted.contains("src/excluded.txt\texcluded file (excludedFiles)"));
    assert!(omitted
        .contains("src/private/hidden.txt\texcluded subdirectory src/private (excludedSubdirs)"));
    assert!(omitted.contains("src/cache/hidden.txt\tglobal directory-name exclude cache"));
    assert!(!omitted.contains("private changed"));
    assert!(archive.by_name("src/excluded.txt").is_err());
    assert!(!status.contains("src/private/hidden.txt"));
    assert!(!status.contains("src/cache/hidden.txt"));

    assert!(
        patch.contains("staged plus final worktree"),
        "captured patch:\n{patch}"
    );
    assert!(patch.contains("diff --git "));
    assert!(patch.contains("@@ -"));
    assert!(patch.contains("unstaged final"));
    assert!(patch.contains("deleted file mode"));
    assert!(patch.contains("-delete me"));
    assert!(patch.contains("rename from src/rename_old.txt"));
    assert!(patch.contains("rename to src/rename_new.txt"));
    assert!(patch.contains("Binary files a/src/blob.dat and b/src/blob.dat differ"));
    assert!(!patch.contains("src/untracked.txt"));
    assert!(!patch.contains("src/excluded.txt"));
    assert!(!patch.contains("src/private/hidden.txt"));
    assert!(!patch.contains("src/cache/hidden.txt"));
    assert!(handoff.contains("captured repository evidence"));
    assert!(archive.by_name("src/deleted.txt").is_err());
    assert!(archive.by_name("src/rename_old.txt").is_err());
    assert!(archive.by_name("src/rename_new.txt").is_ok());
    assert!(archive.by_name("src/untracked.txt").is_ok());
    assert_eq!(
        read_bytes(&mut archive, "src/blob.dat"),
        b"\0binary changed payload\0"
    );

    let uncompressed_total: u64 = (0..archive.len())
        .map(|index| archive.by_index(index).expect("zip entry").size())
        .sum();
    assert_eq!(manifest["totalBytesBestEffort"], uncompressed_total);
}

#[test]
fn selected_ignored_files_are_explicit_untracked_head_evidence() {
    let root = tempdir().expect("tempdir");
    let repo = root.path().join("repo");
    std::fs::create_dir_all(repo.join("src/ignored_dir")).expect("ignored directory");
    std::fs::create_dir_all(repo.join("src/private")).expect("private directory");
    git(&repo, &["init", "-q"]);
    git(&repo, &["config", "user.email", "bundle@example.test"]);
    git(&repo, &["config", "user.name", "Bundle Test"]);
    write(
        &repo,
        ".gitignore",
        b"src/ignored_by_git.txt\nsrc/ignored_dir/\nsrc/private/\n",
    );
    git(&repo, &["add", ".gitignore"]);
    git(&repo, &["commit", "-qm", "baseline"]);
    std::fs::write(repo.join(".git/info/exclude"), "src/ignored_by_info.txt\n")
        .expect("info exclude");
    write(&repo, "src/ignored_by_git.txt", b"selected ignored\n");
    write(&repo, "src/ignored_by_info.txt", b"selected info ignored\n");
    write(
        &repo,
        "src/ignored_dir/nested.txt",
        b"selected ignored descendant\n",
    );
    write(
        &repo,
        "src/private/hidden.txt",
        b"excluded ignored content\n",
    );

    let output = root.path().join("ignored.zip");
    write_bundle(&plan(&repo, &output)).expect("write ignored-file bundle");
    let mut archive = open_zip(&output);
    let manifest = read_json(&mut archive, "BUNDLE_MANIFEST.json");
    let status = read_text(&mut archive, "BUNDLE_GIT_STATUS.txt");
    let patch = read_text(&mut archive, "BUNDLE_GIT_DIFF.patch");

    assert_eq!(manifest["git"]["status"], "complete");
    assert_eq!(manifest["git"]["repoDirty"], false);
    assert_eq!(manifest["git"]["selectionDirty"], true);
    assert_eq!(manifest["git"]["counts"]["selectedChanged"], 3);
    assert_eq!(manifest["git"]["counts"]["selectedTrackedChanged"], 0);
    assert_eq!(manifest["git"]["counts"]["selectedUntracked"], 3);
    assert_eq!(manifest["git"]["counts"]["omittedChangedPaths"], 0);
    assert!(
        status.contains("!! src/ignored_by_git.txt [untracked; ignored by Git; no HEAD ancestor]")
    );
    assert!(
        status.contains("!! src/ignored_by_info.txt [untracked; ignored by Git; no HEAD ancestor]")
    );
    assert!(status
        .contains("!! src/ignored_dir/nested.txt [untracked; ignored by Git; no HEAD ancestor]"));
    assert!(!status.contains("src/private/hidden.txt"));
    assert!(patch.is_empty());
    assert!(archive.by_name("src/ignored_by_git.txt").is_ok());
    assert!(archive.by_name("src/ignored_by_info.txt").is_ok());
    assert!(archive.by_name("src/ignored_dir/nested.txt").is_ok());
    assert!(archive.by_name("src/private/hidden.txt").is_err());
}

#[test]
fn clean_and_non_git_repos_emit_truthful_evidence() {
    let root = tempdir().expect("tempdir");
    let repo = root.path().join("clean");
    std::fs::create_dir_all(repo.join("src")).expect("repo files");
    git(&repo, &["init", "-q"]);
    git(&repo, &["config", "user.email", "bundle@example.test"]);
    git(&repo, &["config", "user.name", "Bundle Test"]);
    write(&repo, "src/file.txt", b"clean\n");
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-qm", "baseline"]);

    let clean_output = root.path().join("clean.zip");
    write_bundle(&plan(&repo, &clean_output)).expect("clean bundle");
    let mut clean = open_zip(&clean_output);
    let clean_manifest = read_json(&mut clean, "BUNDLE_MANIFEST.json");
    assert_eq!(clean_manifest["git"]["status"], "complete");
    assert_eq!(clean_manifest["git"]["repoDirty"], false);
    assert_eq!(clean_manifest["git"]["selectionDirty"], false);
    assert!(read_text(&mut clean, "BUNDLE_GIT_DIFF.patch").is_empty());
    assert!(read_text(&mut clean, "BUNDLE_GIT_STATUS.txt").contains("Clean:"));
    assert!(read_text(&mut clean, "BUNDLE_GIT_OMITTED_PATHS.txt").contains("(none:"));
    assert!(read_text(&mut clean, "BUNDLE_GIT_INDEX_DIFF.patch").is_empty());
    assert!(read_text(&mut clean, "BUNDLE_GIT_WORKTREE_DIFF.patch").is_empty());

    write(&repo, "src/excluded.txt", b"excluded-only change\n");
    let excluded_output = root.path().join("excluded-only.zip");
    write_bundle(&plan(&repo, &excluded_output)).expect("excluded-only bundle");
    let mut excluded_only = open_zip(&excluded_output);
    let excluded_manifest = read_json(&mut excluded_only, "BUNDLE_MANIFEST.json");
    assert_eq!(excluded_manifest["git"]["repoDirty"], true);
    assert_eq!(excluded_manifest["git"]["selectionDirty"], false);
    assert_eq!(excluded_manifest["git"]["counts"]["omittedChangedPaths"], 1);
    assert!(read_text(&mut excluded_only, "BUNDLE_GIT_DIFF.patch").is_empty());

    let plain = root.path().join("plain");
    std::fs::create_dir_all(plain.join("src")).expect("plain repo");
    write(&plain, "src/file.txt", b"not git\n");
    let plain_output = root.path().join("plain.zip");
    write_bundle(&plan(&plain, &plain_output)).expect("non-git bundle");
    let mut non_git = open_zip(&plain_output);
    let manifest = read_json(&mut non_git, "BUNDLE_MANIFEST.json");
    assert_eq!(manifest["git"]["status"], "unavailable");
    assert_eq!(manifest["git"]["issues"][0]["kind"], "notGitRepository");
    assert!(read_text(&mut non_git, "BUNDLE_GIT_DIFF.patch").is_empty());
}

#[test]
fn reserved_generated_entry_collision_fails_closed() {
    let root = tempdir().expect("tempdir");
    let repo = root.path().join("repo");
    std::fs::create_dir_all(&repo).expect("repo");
    write(
        &repo,
        "BUNDLE_GIT_STATUS.txt",
        b"repository-owned collision\n",
    );
    let error = write_bundle(&BundlePlan {
        output_path: root.path().join("collision.zip"),
        repo_root: repo,
        repo_id: "repo".to_string(),
        preset_id: "context".to_string(),
        preset_name: "Context".to_string(),
        selection: BundleSelection {
            include_root: true,
            top_level_dirs: vec![],
            included_subdirs: vec![],
            excluded_subdirs: vec![],
            excluded_files: vec![],
        },
        built_at_iso: "2026-07-10T12:00:00Z".to_string(),
        global_excludes: GlobalExcludes::default(),
    })
    .expect_err("reserved collision must fail");
    assert!(matches!(
        error,
        im_bundle::BundleError::ReservedEntryCollision { .. }
    ));
}

#[cfg(unix)]
#[test]
fn non_utf8_git_paths_are_losslessly_quoted_in_evidence() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let root = tempdir().expect("tempdir");
    let repo = root.path().join("repo");
    std::fs::create_dir_all(repo.join("src")).expect("repo");
    git(&repo, &["init", "-q"]);
    git(&repo, &["config", "user.email", "bundle@example.test"]);
    git(&repo, &["config", "user.name", "Bundle Test"]);
    let unusual = repo
        .join("src")
        .join(OsString::from_vec(b"odd-\xff.txt".to_vec()));
    std::fs::write(&unusual, "baseline\n").expect("baseline");
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-qm", "baseline"]);
    std::fs::write(&unusual, "changed\n").expect("change unusual path");

    let output = root.path().join("unusual.zip");
    write_bundle(&plan(&repo, &output)).expect("unusual path bundle");
    let mut archive = open_zip(&output);
    let manifest = read_json(&mut archive, "BUNDLE_MANIFEST.json");
    let status = read_text(&mut archive, "BUNDLE_GIT_STATUS.txt");
    let patch = read_text(&mut archive, "BUNDLE_GIT_DIFF.patch");
    assert_eq!(manifest["git"]["status"], "complete");
    assert!(status.contains("src/odd-\\xff.txt"));
    assert!(patch.contains("odd-\\377.txt"));
}

fn plan(repo: &Path, output: &Path) -> BundlePlan {
    BundlePlan {
        output_path: output.to_path_buf(),
        repo_root: repo.to_path_buf(),
        repo_id: "repo".to_string(),
        preset_id: "context".to_string(),
        preset_name: "Context".to_string(),
        selection: BundleSelection {
            include_root: false,
            top_level_dirs: vec!["src".to_string()],
            included_subdirs: vec![],
            excluded_subdirs: vec!["src/private".to_string()],
            excluded_files: vec!["src/excluded.txt".to_string()],
        },
        built_at_iso: "2026-07-10T12:00:00Z".to_string(),
        global_excludes: GlobalExcludes {
            dir_names: vec!["cache".to_string()],
            dir_suffixes: vec![],
            file_names: vec![],
            extensions: vec![],
            patterns: vec![],
        },
    }
}

fn git_stdout(repo: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .expect("run git");
    assert!(output.status.success(), "git command failed: {args:?}");
    String::from_utf8(output.stdout)
        .expect("utf8 git output")
        .trim()
        .to_string()
}

fn git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(repo)
        .status()
        .expect("run git");
    assert!(status.success(), "git command failed: {args:?}");
}

fn write(repo: &Path, relative: &str, contents: &[u8]) {
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
