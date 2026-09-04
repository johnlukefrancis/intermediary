// Path: crates/im_agent/src/repos/import/tests_refusals.rs
// Description: Import refusal tests: every error the wire contract names, and the proof nothing was written

use std::fs;

use tempfile::tempdir;

use crate::protocol::ImportConflictPolicy;

use super::tests_support::{import, outside_file, worktree};
use super::MAX_IMPORT_ENTRIES;

#[tokio::test]
async fn two_sources_with_one_basename_are_refused_under_either_policy() {
    let repo = worktree();
    let left = tempdir().expect("left");
    let right = tempdir().expect("right");
    let sources = [
        outside_file(left.path(), "same.txt", "l"),
        outside_file(right.path(), "same.txt", "r"),
    ];

    for policy in [
        ImportConflictPolicy::Refuse,
        ImportConflictPolicy::Replace(vec!["app/same.txt".to_string()]),
    ] {
        let error = import(repo.path(), "app", &sources, policy)
            .await
            .expect_err("duplicate");
        assert_eq!(error.code(), "ENTRY_CONFLICT");
        assert_eq!(
            error.details().and_then(|details| details.get("conflicts")),
            Some(&serde_json::json!(["app/same.txt"]))
        );
        assert!(!repo.path().join("app/same.txt").exists());
    }
}

#[tokio::test]
async fn a_missing_source_is_reported_as_missing() {
    let repo = worktree();
    let source_dir = tempdir().expect("source dir");
    let missing = source_dir.path().join("gone.txt").to_string_lossy().to_string();

    let error = import(repo.path(), "app", &[missing], ImportConflictPolicy::Refuse)
        .await
        .expect_err("missing");

    assert_eq!(error.code(), "IMPORT_SOURCE_NOT_FOUND");
    assert_eq!(error.effect(), Some("notApplied"));
}

#[tokio::test]
async fn unsupported_sources_are_refused_before_anything_is_written() {
    let repo = worktree();
    let source_dir = tempdir().expect("source dir");
    let git_dir = source_dir.path().join(".git");
    fs::create_dir_all(&git_dir).expect("git dir");
    let mut unsupported = vec![
        "relative/path.txt".to_string(),
        String::new(),
        git_dir.to_string_lossy().to_string(),
        // The destination folder itself, which would copy into its own child.
        repo.path().join("app").to_string_lossy().to_string(),
    ];
    #[cfg(unix)]
    {
        let link = source_dir.path().join("link.txt");
        outside_file(source_dir.path(), "real.txt", "r");
        std::os::unix::fs::symlink(source_dir.path().join("real.txt"), &link).expect("symlink");
        unsupported.push(link.to_string_lossy().to_string());
    }

    for source in unsupported {
        let error = import(
            repo.path(),
            "app",
            std::slice::from_ref(&source),
            ImportConflictPolicy::Refuse,
        )
        .await
        .expect_err("unsupported");
        assert_eq!(error.code(), "IMPORT_UNSUPPORTED_SOURCE", "{source}");
        assert_eq!(error.effect(), Some("notApplied"), "{source}");
    }
}

#[tokio::test]
async fn a_source_already_at_the_destination_is_refused() {
    let repo = worktree();
    fs::write(repo.path().join("app/here.txt"), "here").expect("existing");

    let error = import(
        repo.path(),
        "app",
        &[repo.path().join("app/here.txt").to_string_lossy().to_string()],
        ImportConflictPolicy::Replace(Vec::new()),
    )
    .await
    .expect_err("self import");

    assert_eq!(error.code(), "IMPORT_UNSUPPORTED_SOURCE");
    assert_eq!(
        fs::read_to_string(repo.path().join("app/here.txt")).expect("read"),
        "here"
    );
}

#[tokio::test]
async fn the_destination_directory_must_exist_inside_the_worktree() {
    let repo = worktree();
    let source_dir = tempdir().expect("source dir");
    let source = outside_file(source_dir.path(), "a.txt", "a");
    fs::write(repo.path().join("file.txt"), "f").expect("file");

    for (directory, code) in [
        ("../outside", "INVALID_PATH"),
        ("app/missing", "DIR_NOT_FOUND"),
        ("file.txt", "NOT_DIRECTORY"),
    ] {
        let error = import(
            repo.path(),
            directory,
            std::slice::from_ref(&source),
            ImportConflictPolicy::Refuse,
        )
        .await
        .expect_err("bad directory");
        assert_eq!(error.code(), code, "{directory}");
        assert_eq!(error.effect(), Some("notApplied"), "{directory}");
    }
}

#[tokio::test]
async fn a_drop_past_the_entry_cap_is_refused_whole() {
    let repo = worktree();
    let source_dir = tempdir().expect("source dir");
    let tree = source_dir.path().join("huge");
    fs::create_dir_all(&tree).expect("tree");
    for index in 0..MAX_IMPORT_ENTRIES {
        fs::write(tree.join(format!("{index}.txt")), "").expect("write");
    }

    let error = import(
        repo.path(),
        "app",
        &[tree.to_string_lossy().to_string()],
        ImportConflictPolicy::Refuse,
    )
    .await
    .expect_err("too large");

    assert_eq!(error.code(), "IMPORT_TOO_LARGE");
    assert_eq!(error.effect(), Some("notApplied"));
    assert!(!repo.path().join("app/huge").exists());
}

/// The repository's own Git directory is never a destination, at any depth and
/// whichever case the filesystem spells it in. Every one of these is refused
/// before the destination is even resolved, so nothing lands anywhere.
#[tokio::test]
async fn the_git_directory_is_never_a_destination() {
    let repo = worktree();
    let source_dir = tempdir().expect("source dir");
    let source = outside_file(source_dir.path(), "a.txt", "a");
    fs::create_dir_all(repo.path().join(".git/hooks")).expect("git dir");
    fs::create_dir_all(repo.path().join("app/.GIT")).expect("nested git dir");

    for directory in [".git", ".git/hooks", "app/.GIT"] {
        let error = import(
            repo.path(),
            directory,
            std::slice::from_ref(&source),
            ImportConflictPolicy::Refuse,
        )
        .await
        .expect_err("git destination");
        assert_eq!(error.code(), "INVALID_PATH", "{directory}");
        assert_eq!(error.effect(), Some("notApplied"), "{directory}");
        assert!(
            !repo.path().join(directory).join("a.txt").exists(),
            "{directory}"
        );
    }
}

/// A dropped folder carrying a Git directory would plant a second repository
/// inside this worktree. The walk that finds it is still planning, so the
/// whole drop is refused with nothing written — and the message names the
/// folder the user dropped and where inside it the problem is.
#[tokio::test]
async fn a_dropped_folder_carrying_a_git_directory_is_refused_whole() {
    let repo = worktree();
    let source_dir = tempdir().expect("source dir");
    let tree = source_dir.path().join("project");
    fs::create_dir_all(tree.join("deep/.git/objects")).expect("nested git dir");
    fs::write(tree.join("a.txt"), "a").expect("write a");

    let error = import(
        repo.path(),
        "app",
        &[tree.to_string_lossy().to_string()],
        ImportConflictPolicy::Refuse,
    )
    .await
    .expect_err("nested git directory");

    assert_eq!(error.code(), "INVALID_PATH");
    assert_eq!(error.effect(), Some("notApplied"));
    assert!(
        error.message().contains("deep/.git"),
        "the message names where inside the drop it is: {}",
        error.message()
    );
    assert!(!repo.path().join("app/project").exists());
}
