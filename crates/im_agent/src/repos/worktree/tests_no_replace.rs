// Path: crates/im_agent/src/repos/worktree/tests_no_replace.rs
// Description: Tests for the no-replace write a move performs at every destination the user did not authorize

use crate::protocol::ImportConflictPolicy::{Refuse, Replace};

use super::move_entries::move_failure;
use super::tests_support::{act, move_action, read, worktree, write};

/// The move twin of the import's authorization rule. The user authorized
/// replacing `docs/a.txt`; `docs/b.txt` filled up while the dialog was open
/// and was in no list anyone answered, so the whole move is refused and the
/// fresh collision set — both paths — comes back for the next answer.
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

/// Two entries that differ only by case are two destinations on a
/// case-sensitive volume, and this test pins that: both land, and nothing here
/// probes the filesystem to find out which kind it is. On a case-insensitive
/// volume they are one destination reached by two spellings; the second rename
/// then meets the first and the no-replace primitive refuses it as an
/// `ENTRY_CONFLICT` instead of overwriting it.
#[cfg(unix)]
#[tokio::test]
async fn two_entries_differing_only_by_case_both_land_on_a_case_sensitive_volume() {
    let repo = worktree();
    write(repo.path(), "app/A.txt", "upper");
    write(repo.path(), "app/a.txt", "lower");

    let entries = act(
        repo.path(),
        move_action(&["app/A.txt", "app/a.txt"], "docs", Refuse),
    )
    .await
    .expect("move");

    assert_eq!(
        entries,
        vec!["docs/A.txt".to_string(), "docs/a.txt".to_string()]
    );
    assert_eq!(read(repo.path(), "docs/A.txt"), "upper");
    assert_eq!(read(repo.path(), "docs/a.txt"), "lower");
}

/// What the filesystem answered, turned into what the UI is told. An occupied
/// destination the no-replace rename lost to is a conflict naming that path,
/// and once an earlier entry has landed the action is half-applied, so the
/// effect is unknown and `details.applied` says what did move.
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
