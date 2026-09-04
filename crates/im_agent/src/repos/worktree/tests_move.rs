// Path: crates/im_agent/src/repos/worktree/tests_move.rs
// Description: Move behaviour tests: what lands, what is refused whole, and what a folder may never do

use crate::protocol::ImportConflictPolicy::{Refuse, Replace};

use super::tests_support::{act, move_action, read, worktree, write};

#[tokio::test]
async fn a_file_moves_and_the_action_reports_its_new_path() {
    let repo = worktree();
    write(repo.path(), "app/a.txt", "a");

    let entries = act(repo.path(), move_action(&["app/a.txt"], "docs", Refuse))
        .await
        .expect("move");

    assert_eq!(entries, vec!["docs/a.txt".to_string()]);
    assert_eq!(read(repo.path(), "docs/a.txt"), "a");
    assert!(!repo.path().join("app/a.txt").exists());
}

#[tokio::test]
async fn an_empty_directory_means_the_worktree_root() {
    let repo = worktree();
    write(repo.path(), "app/a.txt", "a");

    let entries = act(repo.path(), move_action(&["app/a.txt"], "", Refuse))
        .await
        .expect("move");

    assert_eq!(entries, vec!["a.txt".to_string()]);
    assert!(repo.path().join("a.txt").is_file());
}

#[tokio::test]
async fn refuse_names_the_occupied_destination_and_moves_nothing() {
    let repo = worktree();
    write(repo.path(), "app/a.txt", "new");
    write(repo.path(), "app/b.txt", "fresh");
    write(repo.path(), "docs/a.txt", "old");

    let error = act(
        repo.path(),
        move_action(&["app/a.txt", "app/b.txt"], "docs", Refuse),
    )
    .await
    .expect_err("conflict");

    assert_eq!(error.code(), "ENTRY_CONFLICT");
    assert_eq!(error.effect(), Some("notApplied"));
    assert_eq!(
        error.details().and_then(|details| details.get("conflicts")),
        Some(&serde_json::json!(["docs/a.txt"]))
    );
    assert_eq!(read(repo.path(), "docs/a.txt"), "old");
    assert!(
        repo.path().join("app/b.txt").exists(),
        "the entry that had no conflict is not moved either"
    );
}

#[tokio::test]
async fn replace_overwrites_the_occupied_destination() {
    let repo = worktree();
    write(repo.path(), "app/a.txt", "new");
    write(repo.path(), "docs/a.txt", "old");

    let entries = act(
        repo.path(),
        move_action(&["app/a.txt"], "docs", Replace(vec!["docs/a.txt".to_string()])),
    )
    .await
    .expect("move");

    assert_eq!(entries, vec!["docs/a.txt".to_string()]);
    assert_eq!(read(repo.path(), "docs/a.txt"), "new");
    assert!(!repo.path().join("app/a.txt").exists());
}

/// A folder move is a rename of a whole tree. Landing it on an existing folder
/// of the same name could only replace that tree or merge into it, and neither
/// is what the user asked for by dragging one folder, so both policies refuse.
#[tokio::test]
async fn a_folder_onto_an_existing_folder_is_refused_under_either_policy() {
    let repo = worktree();
    write(repo.path(), "app/assets/a.png", "png");
    write(repo.path(), "docs/assets/kept.txt", "kept");

    for policy in [Refuse, Replace(vec!["docs/assets".to_string()])] {
        let error = act(repo.path(), move_action(&["app/assets"], "docs", policy))
            .await
            .expect_err("folder conflict");

        assert_eq!(error.code(), "ENTRY_CONFLICT");
        assert_eq!(error.effect(), Some("notApplied"));
        assert_eq!(
            error.details().and_then(|details| details.get("conflicts")),
            Some(&serde_json::json!(["docs/assets"]))
        );
        assert_eq!(read(repo.path(), "docs/assets/kept.txt"), "kept");
        assert!(repo.path().join("app/assets/a.png").is_file());
    }
}

#[tokio::test]
async fn a_kind_mismatch_is_refused_under_either_policy() {
    let repo = worktree();
    write(repo.path(), "app/notes", "text");
    std::fs::create_dir_all(repo.path().join("docs/notes")).expect("existing folder");

    for policy in [Refuse, Replace(vec!["docs/notes".to_string()])] {
        let error = act(repo.path(), move_action(&["app/notes"], "docs", policy))
            .await
            .expect_err("kind mismatch");

        assert_eq!(error.code(), "ENTRY_KIND_MISMATCH");
        assert_eq!(error.effect(), Some("notApplied"));
        assert!(repo.path().join("docs/notes").is_dir());
    }
}

#[tokio::test]
async fn a_folder_cannot_be_moved_into_itself_or_below_itself() {
    let repo = worktree();
    write(repo.path(), "app/assets/deep/a.png", "png");

    for directory in ["app/assets", "app/assets/deep"] {
        let error = act(repo.path(), move_action(&["app/assets"], directory, Refuse))
            .await
            .expect_err("self move");

        assert_eq!(error.code(), "INVALID_PATH", "{directory}");
        assert_eq!(error.effect(), Some("notApplied"), "{directory}");
        assert!(repo.path().join("app/assets/deep/a.png").is_file());
    }
}

/// The entry is already in the folder it was dropped on. Nothing moves, but it
/// is still one of the paths this action produced, so it is reported like the
/// rest rather than vanishing from the answer.
#[tokio::test]
async fn an_entry_already_in_the_destination_is_a_reported_no_op() {
    let repo = worktree();
    write(repo.path(), "app/a.txt", "a");

    let entries = act(
        repo.path(),
        move_action(&["app/a.txt"], "app", Replace(Vec::new())),
    )
    .await
    .expect("no-op move");

    assert_eq!(entries, vec!["app/a.txt".to_string()]);
    assert_eq!(read(repo.path(), "app/a.txt"), "a");
}

#[tokio::test]
async fn two_entries_claiming_one_destination_are_refused_whole() {
    let repo = worktree();
    write(repo.path(), "app/a.txt", "one");
    write(repo.path(), "app/deep/a.txt", "two");

    let error = act(
        repo.path(),
        move_action(
            &["app/a.txt", "app/deep/a.txt"],
            "docs",
            Replace(vec!["docs/a.txt".to_string()]),
        ),
    )
    .await
    .expect_err("duplicate destination");

    assert_eq!(error.code(), "ENTRY_CONFLICT");
    assert!(!repo.path().join("docs/a.txt").exists());
}

#[tokio::test]
async fn every_refusal_the_paths_themselves_can_raise_is_proven_before_the_move() {
    let repo = worktree();
    write(repo.path(), "app/a.txt", "a");
    write(repo.path(), "file.txt", "f");

    for (paths, directory, code) in [
        (vec!["app/gone.txt"], "docs", "ENTRY_NOT_FOUND"),
        (vec!["app/.git/config"], "docs", "INVALID_PATH"),
        (vec!["../outside.txt"], "docs", "INVALID_PATH"),
        (vec!["app/a.txt"], "../outside", "INVALID_PATH"),
        (vec!["app/a.txt"], ".git", "INVALID_PATH"),
        (vec!["app/a.txt"], "docs/missing", "DIR_NOT_FOUND"),
        (vec!["app/a.txt"], "file.txt", "NOT_DIRECTORY"),
        (vec![], "docs", "INVALID_PATH"),
    ] {
        let error = act(repo.path(), move_action(&paths, directory, Refuse))
            .await
            .expect_err("refusal");
        assert_eq!(error.code(), code, "{paths:?} -> {directory}");
        assert_eq!(error.effect(), Some("notApplied"), "{paths:?}");
    }
    assert_eq!(read(repo.path(), "app/a.txt"), "a");
}
