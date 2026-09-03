// Path: crates/im_agent/src/source_control/tests_discard_stamps.rs
// Description: Real-git tests binding a discard to the exact file state the user reviewed (stamp, absence, order)

use crate::protocol::{SourceControlActionPayload as Action, SourceControlWorktreeStamp};

use super::tests_support::*;

/// A discard destroys work. The file it destroys must be the file the user
/// confirmed, not whatever an agent wrote a second later.
#[tokio::test]
async fn a_discard_is_refused_when_the_file_changed_since_it_was_reviewed() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "base.txt", b"reviewed edit\n");
    write(&root, "junk.txt", b"junk\n");
    let reviewed = status(&root).await;
    let stamp = reviewed
        .worktree
        .iter()
        .find(|entry| entry.path == "base.txt")
        .and_then(|entry| entry.worktree_stamp)
        .expect("worktree entries carry a stamp");
    assert_eq!(stamp, disk_stamp(&root, "base.txt").expect("stamp"));

    write(&root, "base.txt", b"a coding agent wrote this after the review\n");
    let error = try_act(
        &root,
        Action::Discard {
            targets: vec![
                target("base.txt", Some(stamp)),
                target("junk.txt", disk_stamp(&root, "junk.txt")),
            ],
        },
    )
    .await
    .expect_err("stamp moved");
    assert_eq!(error.code(), "SOURCE_CONTROL_STATE_CHANGED");
    assert!(error.message().starts_with("base.txt changed"), "{}", error.message());
    assert_eq!(error.effect(), Some("notApplied"));
    assert_eq!(
        read(&root, "base.txt"),
        b"a coding agent wrote this after the review\n"
    );
    assert!(root.join("junk.txt").exists(), "one refusal aborts the whole action");
}

/// A file that was already gone at review time carries no stamp: it can be
/// restored from the index, but nothing may be removed for it.
#[tokio::test]
async fn a_target_without_a_stamp_is_restored_and_never_removed() {
    let (_temp, root) = init_repo_with_commit();
    std::fs::remove_file(root.join("base.txt")).expect("delete tracked file");
    let reviewed = status(&root).await;
    assert_eq!(
        reviewed.worktree.first().and_then(|entry| entry.worktree_stamp),
        None
    );

    act(&root, discard_now(&root, &["base.txt"])).await;
    assert_eq!(read(&root, "base.txt"), b"base\n");
}

#[tokio::test]
async fn a_stamp_of_the_wrong_size_is_refused_even_at_the_same_mtime() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "base.txt", b"edited\n");
    let stamp = disk_stamp(&root, "base.txt").expect("stamp");
    let error = try_act(
        &root,
        Action::Discard {
            targets: vec![target(
                "base.txt",
                Some(SourceControlWorktreeStamp {
                    bytes: stamp.bytes + 1,
                    ..stamp
                }),
            )],
        },
    )
    .await
    .expect_err("size moved");
    assert_eq!(error.code(), "SOURCE_CONTROL_STATE_CHANGED");
    assert_eq!(read(&root, "base.txt"), b"edited\n");
}

/// A rewrite that happens to land the same byte count and millisecond still
/// differs at nanosecond resolution, and that alone is enough to refuse.
#[tokio::test]
async fn a_same_length_same_millisecond_rewrite_is_refused_by_its_nanoseconds() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "base.txt", b"edited\n");
    let stamp = disk_stamp(&root, "base.txt").expect("stamp");
    let mismatched_nanos = if stamp.mtime_nanos == 0 { 1 } else { 0 };
    let error = try_act(
        &root,
        Action::Discard {
            targets: vec![target(
                "base.txt",
                Some(SourceControlWorktreeStamp {
                    mtime_nanos: mismatched_nanos,
                    ..stamp
                }),
            )],
        },
    )
    .await
    .expect_err("nanoseconds moved");
    assert_eq!(error.code(), "SOURCE_CONTROL_STATE_CHANGED");
    assert_eq!(read(&root, "base.txt"), b"edited\n");
}

/// A target that vanished between the review and the click cannot be claimed
/// at all. That is the same "changed since it was reviewed" refusal a rewrite
/// gets — nothing moved, and the user's next refresh shows the file is gone —
/// not an internal failure with a stack-shaped message.
#[tokio::test]
async fn a_target_deleted_after_the_review_is_refused_as_changed() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "base.txt", b"edited\n");
    let stamp = disk_stamp(&root, "base.txt").expect("stamp");
    std::fs::remove_file(root.join("base.txt")).expect("delete after the review");

    let error = try_act(
        &root,
        Action::Discard {
            targets: vec![target("base.txt", Some(stamp))],
        },
    )
    .await
    .expect_err("target vanished");
    assert_eq!(error.code(), "SOURCE_CONTROL_STATE_CHANGED");
    assert_eq!(error.message(), "base.txt changed since it was reviewed");
    assert_eq!(error.effect(), Some("notApplied"));
    assert!(!root.join("base.txt").exists(), "nothing was moved or restored");
}

/// A tracked file missing at review, recreated by another writer before the
/// discard runs, must never be silently overwritten by `git restore`.
#[tokio::test]
async fn a_missing_target_recreated_before_discard_is_refused() {
    let (_temp, root) = init_repo_with_commit();
    std::fs::remove_file(root.join("base.txt")).expect("delete tracked file");
    let reviewed = status(&root).await;
    let entry = reviewed
        .worktree
        .iter()
        .find(|entry| entry.path == "base.txt")
        .expect("worktree entry");
    assert!(entry.worktree_missing);
    assert_eq!(entry.worktree_stamp, None);

    write(&root, "base.txt", b"a coding agent recreated this\n");
    let error = try_act(&root, Action::Discard { targets: vec![missing_target("base.txt")] })
        .await
        .expect_err("recreated");
    assert_eq!(error.code(), "SOURCE_CONTROL_STATE_CHANGED");
    assert_eq!(error.effect(), Some("notApplied"));
    assert_eq!(read(&root, "base.txt"), b"a coding agent recreated this\n");
}

/// Two targets, the second refused after the first already landed: the first
/// target's file must stay restored, and the action reports `unknown` (a
/// clean refusal is no longer the whole story once something else changed).
#[tokio::test]
async fn a_second_target_refusal_after_the_first_succeeded_is_unknown_and_names_it() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "base.txt", b"edited\n");
    write(&root, "junk.txt", b"junk\n");
    let first = target("base.txt", disk_stamp(&root, "base.txt"));
    let junk_stamp = disk_stamp(&root, "junk.txt").expect("stamp");
    let second = target(
        "junk.txt",
        Some(SourceControlWorktreeStamp {
            bytes: junk_stamp.bytes + 1,
            ..junk_stamp
        }),
    );
    let error = try_act(&root, Action::Discard { targets: vec![first, second] })
        .await
        .expect_err("second target refused");
    assert_eq!(error.code(), "SOURCE_CONTROL_STATE_CHANGED");
    assert_eq!(error.effect(), Some("unknown"));
    assert!(error.message().contains("already discarded: base.txt"), "{}", error.message());
    assert_eq!(read(&root, "base.txt"), b"base\n", "the first target stays applied");
    assert_eq!(read(&root, "junk.txt"), b"junk\n", "the second target's mismatch rolled back");
}
