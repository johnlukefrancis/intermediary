// Path: crates/im_agent/src/repos/import/tests.rs
// Description: Import behaviour tests: what lands in the worktree under each conflict policy

use std::fs;
use std::path::PathBuf;

use tempfile::tempdir;

use crate::protocol::{ImportConflictPolicy, ImportedFile};

use super::tests_support::{import, outside_file, worktree};

#[tokio::test]
async fn copies_a_file_into_a_subdirectory_and_reports_its_repo_path() {
    let repo = worktree();
    let source_dir = tempdir().expect("source dir");
    let source = outside_file(source_dir.path(), "notes.md", "hello");

    let imported = import(
        repo.path(),
        "app",
        &[source],
        ImportConflictPolicy::Refuse,
    )
    .await
    .expect("import");

    assert_eq!(
        imported,
        vec![ImportedFile {
            path: "app/notes.md".to_string(),
            bytes: 5
        }]
    );
    assert_eq!(
        fs::read_to_string(repo.path().join("app/notes.md")).expect("read"),
        "hello"
    );
}

#[tokio::test]
async fn an_empty_directory_string_means_the_worktree_root() {
    let repo = worktree();
    let source_dir = tempdir().expect("source dir");
    let source = outside_file(source_dir.path(), "a.txt", "a");

    for directory in ["", "."] {
        let repo_dir = tempdir().expect("repo");
        let imported = import(
            repo_dir.path(),
            directory,
            std::slice::from_ref(&source),
            ImportConflictPolicy::Refuse,
        )
        .await
        .expect("import");
        assert_eq!(imported[0].path, "a.txt", "{directory:?}");
    }
    drop(repo);
}

#[tokio::test]
async fn a_directory_source_lands_whole_and_skips_symlinked_entries() {
    let repo = worktree();
    let source_dir = tempdir().expect("source dir");
    let tree = source_dir.path().join("assets");
    fs::create_dir_all(tree.join("icons")).expect("nested dir");
    fs::create_dir_all(tree.join("empty")).expect("empty dir");
    fs::write(tree.join("icons/a.png"), "png").expect("write nested");
    #[cfg(unix)]
    std::os::unix::fs::symlink(source_dir.path(), tree.join("loop")).expect("symlink");

    let imported = import(
        repo.path(),
        "app",
        &[tree.to_string_lossy().to_string()],
        ImportConflictPolicy::Refuse,
    )
    .await
    .expect("import");

    assert_eq!(
        imported,
        vec![ImportedFile {
            path: "app/assets/icons/a.png".to_string(),
            bytes: 3
        }]
    );
    assert!(repo.path().join("app/assets/empty").is_dir());
    assert!(!repo.path().join("app/assets/loop").exists());
}

#[tokio::test]
async fn refuse_names_every_occupied_destination_and_writes_nothing() {
    let repo = worktree();
    let source_dir = tempdir().expect("source dir");
    let taken = outside_file(source_dir.path(), "taken.txt", "new");
    let fresh = outside_file(source_dir.path(), "fresh.txt", "new");
    fs::write(repo.path().join("app/taken.txt"), "old").expect("existing");

    let error = import(
        repo.path(),
        "app",
        &[taken, fresh],
        ImportConflictPolicy::Refuse,
    )
    .await
    .expect_err("conflict");

    assert_eq!(error.code(), "ENTRY_CONFLICT");
    assert_eq!(error.effect(), Some("notApplied"));
    assert_eq!(
        error.details().and_then(|details| details.get("conflicts")),
        Some(&serde_json::json!(["app/taken.txt"]))
    );
    assert_eq!(
        fs::read_to_string(repo.path().join("app/taken.txt")).expect("read"),
        "old"
    );
    assert!(!repo.path().join("app/fresh.txt").exists());
}

#[tokio::test]
async fn replace_overwrites_the_destination_and_leaves_no_temp_file() {
    let repo = worktree();
    let source_dir = tempdir().expect("source dir");
    let source = outside_file(source_dir.path(), "taken.txt", "new bytes");
    fs::write(repo.path().join("app/taken.txt"), "old").expect("existing");

    let imported = import(
        repo.path(),
        "app",
        &[source],
        ImportConflictPolicy::Replace(vec!["app/taken.txt".to_string()]),
    )
    .await
    .expect("import");

    assert_eq!(imported[0].bytes, 9);
    assert_eq!(
        fs::read_to_string(repo.path().join("app/taken.txt")).expect("read"),
        "new bytes"
    );
    let leftovers: Vec<PathBuf> = fs::read_dir(repo.path().join("app"))
        .expect("read dir")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.to_string_lossy().ends_with(".tmp"))
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
}

#[tokio::test]
async fn a_folder_merges_into_an_existing_folder_and_conflicts_per_file() {
    let repo = worktree();
    let source_dir = tempdir().expect("source dir");
    let tree = source_dir.path().join("assets");
    fs::create_dir_all(tree.join("deep")).expect("nested dir");
    fs::write(tree.join("a.png"), "png-a").expect("write a");
    fs::write(tree.join("deep/b.txt"), "b").expect("write b");
    fs::create_dir_all(repo.path().join("app/assets")).expect("existing folder");
    fs::write(repo.path().join("app/assets/a.png"), "old").expect("existing file");
    let source = tree.to_string_lossy().to_string();

    let error = import(repo.path(), "app", &[source.clone()], ImportConflictPolicy::Refuse)
        .await
        .expect_err("per-file conflict");
    assert_eq!(error.code(), "ENTRY_CONFLICT");
    assert_eq!(
        error.details().and_then(|details| details.get("conflicts")),
        Some(&serde_json::json!(["app/assets/a.png"]))
    );
    assert!(!repo.path().join("app/assets/deep").exists());

    let imported = import(
        repo.path(),
        "app",
        &[source],
        ImportConflictPolicy::Replace(vec!["app/assets/a.png".to_string()]),
    )
    .await
    .expect("merge");
    assert_eq!(imported.len(), 2);
    assert_eq!(
        fs::read_to_string(repo.path().join("app/assets/a.png")).expect("read"),
        "png-a"
    );
    assert!(repo.path().join("app/assets/deep/b.txt").is_file());
}

#[tokio::test]
async fn a_kind_mismatch_is_refused_under_either_policy() {
    let repo = worktree();
    let source_dir = tempdir().expect("source dir");
    let file = outside_file(source_dir.path(), "notes", "text");
    fs::create_dir_all(repo.path().join("app/notes")).expect("existing folder");

    for policy in [
        ImportConflictPolicy::Refuse,
        ImportConflictPolicy::Replace(vec!["app/notes".to_string()]),
    ] {
        let error = import(repo.path(), "app", &[file.clone()], policy)
            .await
            .expect_err("kind mismatch");
        assert_eq!(error.code(), "ENTRY_KIND_MISMATCH");
        assert_eq!(error.effect(), Some("notApplied"));
        assert!(repo.path().join("app/notes").is_dir());
    }
}

/// The authorization is the list of paths the user was shown, and nothing
/// else. A destination that filled up while the dialog was open was in no list
/// anyone answered, so the drop is refused — and the refusal carries the whole
/// fresh collision set, because that is what the next authorization has to
/// answer.
#[tokio::test]
async fn a_replace_that_did_not_authorize_a_new_collision_is_refused_with_the_fresh_list() {
    let repo = worktree();
    let source_dir = tempdir().expect("source dir");
    let a = outside_file(source_dir.path(), "a.txt", "new a");
    let b = outside_file(source_dir.path(), "b.txt", "new b");
    fs::write(repo.path().join("app/a.txt"), "old a").expect("reviewed collision");
    fs::write(repo.path().join("app/b.txt"), "old b").expect("collision nobody reviewed");

    let error = import(
        repo.path(),
        "app",
        &[a, b],
        ImportConflictPolicy::Replace(vec!["app/a.txt".to_string()]),
    )
    .await
    .expect_err("unauthorized collision");

    assert_eq!(error.code(), "ENTRY_CONFLICT");
    assert_eq!(error.effect(), Some("notApplied"));
    assert_eq!(
        error.details().and_then(|details| details.get("conflicts")),
        Some(&serde_json::json!(["app/a.txt", "app/b.txt"]))
    );
    assert_eq!(
        fs::read_to_string(repo.path().join("app/a.txt")).expect("read"),
        "old a",
        "the authorized destination is untouched too: the drop was refused whole"
    );
    assert_eq!(
        fs::read_to_string(repo.path().join("app/b.txt")).expect("read"),
        "old b"
    );
}
