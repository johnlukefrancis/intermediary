// Path: crates/im_bundle/src/git_capture/tests.rs
// Description: Failure, timeout, and drift tests for bounded Git capture

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use sha2::{Digest, Sha256};
use tempfile::tempdir;

use crate::plan::{BundlePlan, BundleSelection, GlobalExcludes};
use crate::scanner::ScanEntry;

use super::diff::{FULL_DELETIONS_BUDGET, PATCH_LIMIT};
use super::{
    GitCaptureConfig, GitCaptureSession, GitCaptureState, PatchDeletions, WrittenEntryDigests,
};

#[test]
fn missing_git_executable_yields_unavailable_evidence() {
    let root = tempdir().expect("tempdir");
    let repo = root.path().join("repo");
    std::fs::create_dir_all(repo.join("src")).expect("repo");
    std::fs::write(repo.join("src/file.txt"), "content\n").expect("file");
    let plan = plan(&repo, &root.path().join("out.zip"));
    let session = GitCaptureSession::begin_with_config(
        &plan,
        GitCaptureConfig {
            executable: root.path().join("missing-git"),
            repo_root: repo,
            command_timeout: Duration::from_millis(100),
            patch_limit: PATCH_LIMIT,
            full_deletions_budget: FULL_DELETIONS_BUDGET,
        },
        None,
    )
    .expect("best-effort capture");
    let evidence = session
        .finish(&WrittenEntryDigests::new(), None)
        .expect("finish unavailable capture");
    assert_eq!(evidence.manifest.status, GitCaptureState::Unavailable);
    assert_eq!(evidence.manifest.issues[0].kind, "gitUnavailable");
    assert!(evidence.diff.is_empty());
}

#[cfg(unix)]
#[test]
fn timed_out_git_command_yields_unavailable_evidence() {
    use std::os::unix::fs::PermissionsExt;

    let root = tempdir().expect("tempdir");
    let repo = root.path().join("repo");
    std::fs::create_dir_all(repo.join("src")).expect("repo");
    let fake_git = root.path().join("slow-git");
    std::fs::write(&fake_git, "#!/bin/sh\nexec sleep 2\n").expect("fake git");
    let mut permissions = std::fs::metadata(&fake_git)
        .expect("metadata")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&fake_git, permissions).expect("permissions");

    let plan = plan(&repo, &root.path().join("out.zip"));
    let session = GitCaptureSession::begin_with_config(
        &plan,
        GitCaptureConfig {
            executable: fake_git,
            repo_root: repo,
            command_timeout: Duration::from_millis(20),
            patch_limit: PATCH_LIMIT,
            full_deletions_budget: FULL_DELETIONS_BUDGET,
        },
        None,
    )
    .expect("best-effort capture");
    let evidence = session
        .finish(&WrittenEntryDigests::new(), None)
        .expect("finish timed-out capture");
    assert_eq!(evidence.manifest.status, GitCaptureState::Unavailable);
    assert_eq!(evidence.manifest.issues[0].kind, "commandTimeout");
}

#[cfg(unix)]
#[test]
fn nonzero_git_command_is_distinct_from_non_git_repository() {
    let root = tempdir().expect("tempdir");
    let repo = root.path().join("repo");
    std::fs::create_dir_all(repo.join("src")).expect("repo");
    let plan = plan(&repo, &root.path().join("out.zip"));
    let session = GitCaptureSession::begin_with_config(
        &plan,
        GitCaptureConfig {
            executable: PathBuf::from("/bin/false"),
            repo_root: repo,
            command_timeout: Duration::from_millis(100),
            patch_limit: PATCH_LIMIT,
            full_deletions_budget: FULL_DELETIONS_BUDGET,
        },
        None,
    )
    .expect("best-effort capture");
    let evidence = session
        .finish(&WrittenEntryDigests::new(), None)
        .expect("finish failed-command capture");
    assert_eq!(evidence.manifest.status, GitCaptureState::Unavailable);
    assert_eq!(evidence.manifest.issues[0].kind, "commandFailure");
}

#[test]
fn oversized_deletion_bodies_degrade_to_header_only_patch() {
    let root = tempdir().expect("tempdir");
    let repo = root.path().join("repo");
    std::fs::create_dir_all(repo.join("src")).expect("repo");
    init_git(&repo);
    let removed_body = "removed line that must not survive header-only capture\n".repeat(200);
    std::fs::write(repo.join("src/removed.txt"), &removed_body).expect("removed baseline");
    std::fs::write(repo.join("src/kept.txt"), b"baseline\n").expect("kept baseline");
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-qm", "baseline"]);
    std::fs::remove_file(repo.join("src/removed.txt")).expect("delete tracked file");
    let kept = b"kept but edited\n";
    std::fs::write(repo.join("src/kept.txt"), kept).expect("edit kept file");

    let plan = plan(&repo, &root.path().join("out.zip"));
    let session = GitCaptureSession::begin_with_config(
        &plan,
        GitCaptureConfig {
            executable: PathBuf::from("git"),
            repo_root: repo,
            command_timeout: Duration::from_secs(5),
            patch_limit: PATCH_LIMIT,
            full_deletions_budget: 1024,
        },
        None,
    )
    .expect("begin capture");
    let mut written = WrittenEntryDigests::new();
    written.insert(PathBuf::from("src/kept.txt"), Sha256::digest(kept).into());
    let evidence = session.finish(&written, None).expect("finish capture");

    assert_eq!(evidence.manifest.status, GitCaptureState::Complete);
    assert!(evidence.manifest.issues.is_empty());
    assert_eq!(
        evidence.manifest.patch_deletions,
        PatchDeletions::HeaderOnly
    );
    let patch = String::from_utf8(evidence.diff).expect("utf8 patch");
    assert!(patch.contains("deleted file mode"));
    assert!(!patch.contains("removed line that must not survive"));
    assert!(patch.contains("+kept but edited"));
    let status = String::from_utf8(evidence.status).expect("utf8 status");
    assert!(status.contains("Patch deletions: header-only"));
    assert!(status.contains("src/removed.txt"));
}

#[test]
fn selected_state_movement_marks_capture_unstable() {
    let root = tempdir().expect("tempdir");
    let repo = root.path().join("repo");
    std::fs::create_dir_all(repo.join("src")).expect("repo");
    init_git(&repo);
    let selected_path = PathBuf::from("src/file.txt");
    let before = b"baseline\n";
    std::fs::write(repo.join(&selected_path), before).expect("baseline");
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-qm", "baseline"]);
    std::fs::write(repo.join(&selected_path), b"first captured state\n").expect("dirty state");

    let plan = plan(&repo, &root.path().join("out.zip"));
    let session = GitCaptureSession::begin(&plan, None).expect("begin capture");
    let mut written = WrittenEntryDigests::new();
    std::fs::write(repo.join("src/file.txt"), b"moved during capture\n").expect("drift");
    written.insert(
        selected_path,
        Sha256::digest(b"moved during capture\n").into(),
    );

    let evidence = session.finish(&written, None).expect("finish capture");
    assert_eq!(evidence.manifest.status, GitCaptureState::Unstable);
    assert!(evidence
        .manifest
        .issues
        .iter()
        .any(|issue| issue.kind == "captureDrift"));
}

#[test]
fn selected_ignore_classification_movement_marks_capture_unstable() {
    let root = tempdir().expect("tempdir");
    let repo = root.path().join("repo");
    std::fs::create_dir_all(repo.join("src")).expect("repo");
    init_git(&repo);
    std::fs::write(repo.join("src/baseline.txt"), "baseline\n").expect("baseline");
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-qm", "baseline"]);
    std::fs::write(repo.join(".git/info/exclude"), "src/ignored.txt\n")
        .expect("initial ignore rule");
    let selected_path = PathBuf::from("src/ignored.txt");
    let selected_bytes = b"ignored selected file\n";
    std::fs::write(repo.join(&selected_path), selected_bytes).expect("ignored file");

    let plan = plan(&repo, &root.path().join("out.zip"));
    let mut session = GitCaptureSession::begin(&plan, None).expect("begin capture");
    session
        .reconcile_selected_files(
            &[ScanEntry {
                source_path: repo.join(&selected_path),
                repo_relative_path: selected_path.clone(),
                archive_path: "src/ignored.txt".to_string(),
            }],
            None,
        )
        .expect("reconcile ignored file");

    std::fs::write(repo.join(".git/info/exclude"), "").expect("move ignore rules");
    let mut written = WrittenEntryDigests::new();
    written.insert(selected_path, Sha256::digest(selected_bytes).into());

    let evidence = session.finish(&written, None).expect("finish capture");
    assert_eq!(evidence.manifest.status, GitCaptureState::Unstable);
    assert!(evidence.manifest.issues.iter().any(|issue| {
        issue.kind == "captureDrift"
            && issue.detail.contains("ignore classification")
            && issue.artifact.as_deref() == Some("BUNDLE_GIT_STATUS.txt")
    }));
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
            excluded_subdirs: vec![],
            excluded_files: vec![],
        },
        built_at_iso: "2026-07-10T12:00:00Z".to_string(),
        global_excludes: GlobalExcludes {
            dir_names: vec![],
            dir_suffixes: vec![],
            file_names: vec![],
            extensions: vec![],
            patterns: vec![],
        },
    }
}

fn init_git(repo: &Path) {
    git(repo, &["init", "-q"]);
    git(repo, &["config", "user.email", "bundle@example.test"]);
    git(repo, &["config", "user.name", "Bundle Test"]);
}

fn git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(repo)
        .status()
        .expect("run git");
    assert!(status.success(), "git command failed: {args:?}");
}
