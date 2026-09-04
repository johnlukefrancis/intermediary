// Path: crates/im_agent/src/repos/worktree/tests_copy.rs
// Description: In-repo copy tests: the import writer's behaviour, reached with repo-relative entries

use crate::protocol::ImportConflictPolicy::{Refuse, Replace};

use super::tests_support::{act, copy_action, read, worktree, write};

#[tokio::test]
async fn a_file_is_copied_and_the_source_stays_where_it_was() {
    let repo = worktree();
    write(repo.path(), "app/a.txt", "a");

    let entries = act(repo.path(), copy_action(&["app/a.txt"], "docs", Refuse))
        .await
        .expect("copy");

    assert_eq!(entries, vec!["docs/a.txt".to_string()]);
    assert_eq!(read(repo.path(), "docs/a.txt"), "a");
    assert_eq!(read(repo.path(), "app/a.txt"), "a");
}

/// Unlike a move, a copied folder merges into an existing folder of the same
/// name: the source tree stays where it is, so nothing is lost by adding to
/// the destination, and per-file conflicts still answer to the policy.
#[tokio::test]
async fn a_folder_merges_into_an_existing_folder_and_conflicts_per_file() {
    let repo = worktree();
    write(repo.path(), "app/assets/a.png", "new");
    write(repo.path(), "app/assets/deep/b.txt", "b");
    write(repo.path(), "docs/assets/a.png", "old");

    let error = act(repo.path(), copy_action(&["app/assets"], "docs", Refuse))
        .await
        .expect_err("per-file conflict");
    assert_eq!(error.code(), "ENTRY_CONFLICT");
    assert_eq!(
        error.details().and_then(|details| details.get("conflicts")),
        Some(&serde_json::json!(["docs/assets/a.png"]))
    );
    assert!(!repo.path().join("docs/assets/deep").exists());

    let entries = act(
        repo.path(),
        copy_action(
            &["app/assets"],
            "docs",
            Replace(vec!["docs/assets/a.png".to_string()]),
        ),
    )
    .await
    .expect("merge");
    assert_eq!(entries, vec!["docs/assets".to_string()]);
    assert_eq!(read(repo.path(), "docs/assets/a.png"), "new");
    assert_eq!(read(repo.path(), "docs/assets/deep/b.txt"), "b");
}

/// Copying an entry into the folder it already sits in would have to write the
/// file it is reading. The import's own self-copy check refuses it, and the
/// entry is untouched.
#[tokio::test]
async fn copying_into_the_folder_it_already_sits_in_is_refused() {
    let repo = worktree();
    write(repo.path(), "app/a.txt", "a");

    let error = act(
        repo.path(),
        copy_action(&["app/a.txt"], "app", Replace(Vec::new())),
    )
    .await
    .expect_err("self copy");

    assert_eq!(error.code(), "IMPORT_UNSUPPORTED_SOURCE");
    assert_eq!(error.effect(), Some("notApplied"));
    assert_eq!(read(repo.path(), "app/a.txt"), "a");
}

#[tokio::test]
async fn the_entry_refusals_are_proven_before_anything_is_copied() {
    let repo = worktree();
    write(repo.path(), "app/a.txt", "a");

    for (paths, directory, code) in [
        (vec!["app/gone.txt"], "docs", "ENTRY_NOT_FOUND"),
        (vec!["app/.git/config"], "docs", "INVALID_PATH"),
        (vec!["app/a.txt"], ".git", "INVALID_PATH"),
        (vec![], "docs", "INVALID_PATH"),
    ] {
        let error = act(repo.path(), copy_action(&paths, directory, Refuse))
            .await
            .expect_err("refusal");
        assert_eq!(error.code(), code, "{paths:?} -> {directory}");
        assert_eq!(error.effect(), Some("notApplied"), "{paths:?}");
    }
    assert!(!repo.path().join("docs/a.txt").exists());
}
