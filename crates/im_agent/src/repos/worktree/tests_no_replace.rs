// Path: crates/im_agent/src/repos/worktree/tests_no_replace.rs
// Description: Tests for the no-replace write a move performs at every destination the user did not authorize

use crate::protocol::ImportConflictPolicy::{Refuse, Replace};

use super::move_entries::move_failure;
use super::tests_support::{act, move_action, read, worktree, write};

/// A new collision invalidates the earlier replacement authorization.
/// The fresh collision list includes both destinations before any move.
#[tokio::test]
async fn a_replace_that_did_not_authorize_a_new_collision_is_refused_with_the_fresh_list() {
    let repo = worktree();
    write(repo.path(), "app/a.txt", "new a");
    write(repo.path(), "app/b.txt", "new b");
    write(repo.path(), "docs/a.txt", "old a");
    write(repo.path(), "docs/b.txt", "old b");

    let error = act(
        repo.path(),
        move_action(
            &["app/a.txt", "app/b.txt"],
            "docs",
            Replace(vec!["docs/a.txt".to_string()]),
        ),
    )
    .await
    .expect_err("unauthorized collision");

    assert_eq!(error.code(), "ENTRY_CONFLICT");
    assert_eq!(error.effect(), Some("notApplied"));
    assert_eq!(
        error.details().and_then(|details| details.get("conflicts")),
        Some(&serde_json::json!(["docs/a.txt", "docs/b.txt"]))
    );
    assert_eq!(read(repo.path(), "docs/a.txt"), "old a");
    assert_eq!(read(repo.path(), "docs/b.txt"), "old b");
    assert_eq!(read(repo.path(), "app/a.txt"), "new a");
    assert_eq!(read(repo.path(), "app/b.txt"), "new b");
}

/// Separate source directories keep both inputs intact on either volume type.
/// Destination aliases must conflict without overwriting the first moved file.
#[cfg(unix)]
#[tokio::test]
async fn case_alias_destinations_never_overwrite_each_other() {
    let repo = worktree();
    write(repo.path(), "app/upper/A.txt", "upper");
    write(repo.path(), "app/lower/a.txt", "lower");
    let case_insensitive = repo.path().join("app/upper/a.txt").exists();

    let result = act(
        repo.path(),
        move_action(&["app/upper/A.txt", "app/lower/a.txt"], "docs", Refuse),
    )
    .await;

    assert_eq!(read(repo.path(), "docs/A.txt"), "upper");
    if case_insensitive {
        let error = result.expect_err("aliased destination must refuse replacement");
        assert_eq!(error.code(), "ENTRY_CONFLICT");
        assert_eq!(error.effect(), Some("unknown"));
        assert_eq!(
            error.details().and_then(|details| details.get("applied")),
            Some(&serde_json::json!(["docs/A.txt"]))
        );
        assert_eq!(read(repo.path(), "app/lower/a.txt"), "lower");
    } else {
        assert_eq!(
            result.expect("distinct destinations"),
            vec!["docs/A.txt".to_string(), "docs/a.txt".to_string()]
        );
        assert_eq!(read(repo.path(), "docs/a.txt"), "lower");
    }
}

/// A lost no-replace race names its conflict and any entries already moved.
/// Partial application has an unknown effect; zero writes are notApplied.
#[test]
fn the_rename_failures_are_classified_by_what_the_filesystem_answered() {
    use std::io::{Error, ErrorKind};

    let landed = move_failure(
        &["docs/a.txt".to_string()],
        "docs/b.txt",
        &Error::from(ErrorKind::AlreadyExists),
    );
    assert_eq!(landed.code(), "ENTRY_CONFLICT");
    assert_eq!(landed.effect(), Some("unknown"));
    assert_eq!(
        landed.details().and_then(|details| details.get("conflicts")),
        Some(&serde_json::json!(["docs/b.txt"]))
    );
    assert_eq!(
        landed.details().and_then(|details| details.get("applied")),
        Some(&serde_json::json!(["docs/a.txt"]))
    );

    let nothing_landed = move_failure(&[], "docs/b.txt", &Error::from(ErrorKind::AlreadyExists));
    assert_eq!(nothing_landed.code(), "ENTRY_CONFLICT");
    assert_eq!(nothing_landed.effect(), Some("notApplied"));

    let unsupported = move_failure(&[], "docs/b.txt", &Error::from(ErrorKind::Unsupported));
    assert_eq!(unsupported.code(), "SOURCE_CONTROL_UNSUPPORTED_LAYOUT");
    assert_eq!(unsupported.effect(), Some("notApplied"));
}
