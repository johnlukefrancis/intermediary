// Path: crates/im_agent/src/repos/worktree/tests_rename.rs
// Description: Rename behaviour tests: what commits, which names are refused, and what is never replaced

use super::entries::destination_is_the_source;
use super::tests_support::{act, read, rename_action, worktree, write};

#[tokio::test]
async fn a_rename_commits_and_reports_the_new_path() {
    let repo = worktree();
    write(repo.path(), "app/a.txt", "a");

    let entries = act(repo.path(), rename_action("app/a.txt", "b.txt"))
        .await
        .expect("rename");

    assert_eq!(entries, vec!["app/b.txt".to_string()]);
    assert_eq!(read(repo.path(), "app/b.txt"), "a");
    assert!(!repo.path().join("app/a.txt").exists());
}

#[tokio::test]
async fn a_folder_renames_whole() {
    let repo = worktree();
    write(repo.path(), "app/assets/a.png", "png");

    let entries = act(repo.path(), rename_action("app/assets", "images"))
        .await
        .expect("rename");

    assert_eq!(entries, vec!["app/images".to_string()]);
    assert_eq!(read(repo.path(), "app/images/a.png"), "png");
}

#[tokio::test]
async fn a_root_level_entry_renames_without_a_parent_prefix() {
    let repo = worktree();
    write(repo.path(), "a.txt", "a");

    let entries = act(repo.path(), rename_action("a.txt", "b.txt"))
        .await
        .expect("rename");

    assert_eq!(entries, vec!["b.txt".to_string()]);
}

/// A new name is one name. Everything that could steer the rename into another
/// folder, out of the worktree, or into the repository's own Git directory is
/// refused before the entry is touched.
#[tokio::test]
async fn every_invalid_name_is_refused_by_name() {
    let repo = worktree();
    write(repo.path(), "app/a.txt", "a");

    for name in ["", "   ", "sub/b.txt", "sub\\b.txt", "b\0.txt", ".", "..", ".git", ".GIT"] {
        let error = act(repo.path(), rename_action("app/a.txt", name))
            .await
            .expect_err("invalid name");
        assert_eq!(error.code(), "ENTRY_INVALID_NAME", "{name:?}");
        assert_eq!(error.effect(), Some("notApplied"), "{name:?}");
    }
    assert_eq!(read(repo.path(), "app/a.txt"), "a");
}

/// A rename has no policy: the gesture that produces it carries no answer to
/// "and destroy what is already called that?", so an occupied destination is
/// always refused and named.
#[tokio::test]
async fn an_occupied_destination_is_refused_and_never_replaced() {
    let repo = worktree();
    write(repo.path(), "app/a.txt", "a");
    write(repo.path(), "app/b.txt", "b");

    let error = act(repo.path(), rename_action("app/a.txt", "b.txt"))
        .await
        .expect_err("conflict");

    assert_eq!(error.code(), "ENTRY_CONFLICT");
    assert_eq!(error.effect(), Some("notApplied"));
    assert_eq!(
        error.details().and_then(|details| details.get("conflicts")),
        Some(&serde_json::json!(["app/b.txt"]))
    );
    assert_eq!(read(repo.path(), "app/b.txt"), "b");
    assert_eq!(read(repo.path(), "app/a.txt"), "a");
}

#[tokio::test]
async fn renaming_something_that_is_gone_says_so() {
    let repo = worktree();

    let error = act(repo.path(), rename_action("app/gone.txt", "b.txt"))
        .await
        .expect_err("missing");

    assert_eq!(error.code(), "ENTRY_NOT_FOUND");
    assert_eq!(error.effect(), Some("notApplied"));
}

/// The predicate that makes a case-only rename possible on a case-insensitive
/// filesystem: the destination is occupied by the very file being renamed.
/// On a case-sensitive tmpdir the two names are two different paths, which is
/// exactly the answer it must give there.
#[tokio::test]
async fn the_destination_is_the_source_only_when_it_is_the_same_file() {
    let repo = worktree();
    write(repo.path(), "app/a.txt", "a");
    let source = repo.path().join("app/a.txt");

    assert!(destination_is_the_source(&source, &source).await);
    assert!(!destination_is_the_source(&source, &repo.path().join("app/b.txt")).await);
}

/// A rename that only changes case must survive a filesystem that reports the
/// destination as already taken; on a case-sensitive one it is an ordinary
/// rename onto a free path. Both filesystems must end with the new spelling.
#[tokio::test]
async fn a_case_only_rename_is_never_a_conflict_with_itself() {
    let repo = worktree();
    write(repo.path(), "app/notes.md", "n");

    let entries = act(repo.path(), rename_action("app/notes.md", "Notes.md"))
        .await
        .expect("case-only rename");

    assert_eq!(entries, vec!["app/Notes.md".to_string()]);
    assert_eq!(read(repo.path(), "app/Notes.md"), "n");
}

/// The other direction of the case-only rename, and the one that exercises the
/// no-replace primitive: on a case-sensitive volume `Notes.md` and `notes.md`
/// are two paths, so the destination is genuinely free and the rename that
/// cannot replace lands on it.
#[cfg(unix)]
#[tokio::test]
async fn a_case_only_rename_still_lands() {
    let repo = worktree();
    write(repo.path(), "app/Notes.md", "n");

    let entries = act(repo.path(), rename_action("app/Notes.md", "notes.md"))
        .await
        .expect("case-only rename");

    assert_eq!(entries, vec!["app/notes.md".to_string()]);
    assert_eq!(read(repo.path(), "app/notes.md"), "n");
}
