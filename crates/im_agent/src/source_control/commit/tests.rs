// Path: crates/im_agent/src/source_control/commit/tests.rs
// Description: Real-git tests for the commit oracle, its snapshot precondition, and the landed-but-unread error

use crate::protocol::{SourceControlChange::*, SourceControlEntryArea::*};

use crate::source_control::tests_support::*;

#[tokio::test]
async fn commit_returns_the_new_head_and_clears_the_index() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "base.txt", b"changed\n");
    git(&root, &["add", "base.txt"]);
    let action = commit_now(&root, "  Change base\n\n").await;
    let outcome = act(&root, action).await;
    let head = git_stdout(&root, &["rev-parse", "HEAD"]);
    assert_eq!(outcome.commit_sha.as_deref(), Some(head.as_str()));
    assert_eq!(outcome.status.head_sha.as_deref(), Some(head.as_str()));
    assert!(outcome.status.index.is_empty());
    assert!(outcome.status.worktree.is_empty());
    assert_eq!(
        git_stdout(&root, &["log", "-1", "--format=%s"]),
        "Change base"
    );
}

#[tokio::test]
async fn commit_with_nothing_staged_or_a_blank_message_is_refused() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "base.txt", b"unstaged change\n");
    let action = commit_now(&root, "message").await;
    let error = try_act(&root, action).await.expect_err("nothing staged");
    assert_eq!(error.code(), "GIT_NOTHING_TO_COMMIT");

    git(&root, &["add", "base.txt"]);
    let action = commit_now(&root, "  \n").await;
    let error = try_act(&root, action).await.expect_err("blank message");
    assert_eq!(error.code(), "INVALID_COMMIT_MESSAGE");
    assert_eq!(
        status(&root).await.index,
        vec![entry("base.txt", Index, Modified)]
    );
}

#[tokio::test]
async fn merge_resolved_to_head_is_committable_and_commits_with_two_parents() {
    let (_temp, root) = init_repo_with_commit();
    git(&root, &["checkout", "-q", "-b", "feature"]);
    write(&root, "base.txt", b"feature\n");
    git(&root, &["commit", "-qam", "feature"]);
    git(&root, &["checkout", "-q", "main"]);
    write(&root, "base.txt", b"main\n");
    git(&root, &["commit", "-qam", "main"]);
    assert!(!git_succeeds(&root, &["merge", "-q", "feature"]));
    let too_early = commit_now(&root, "too early").await;
    let refused = try_act(&root, too_early)
        .await
        .expect_err("unmerged paths block the commit");
    assert_eq!(refused.code(), "GIT_UNMERGED_PATHS");
    // Resolve by keeping HEAD's content: the index now equals HEAD, yet the
    // merge must still be concluded by a commit.
    write(&root, "base.txt", b"main\n");
    git(&root, &["add", "base.txt"]);

    let status = status(&root).await;
    assert!(status.index.is_empty() && status.conflicts.is_empty());
    assert!(status.committable, "a merge in progress is committable");

    let action = commit_now(&root, "Merge feature").await;
    let outcome = act(&root, action).await;
    let head = git_stdout(&root, &["rev-parse", "HEAD"]);
    assert_eq!(outcome.commit_sha.as_deref(), Some(head.as_str()));
    let parents = git_stdout(&root, &["rev-list", "--parents", "-1", "HEAD"]);
    assert_eq!(
        parents.split_whitespace().count(),
        3,
        "two parents: {parents}"
    );
    assert!(!outcome.status.committable);
}

/// HEAD is part of the reviewed snapshot: a commit that landed on a
/// terminal/agent's own click between the review and this click must be
/// refused before this click adds a second, unreviewed commit on top.
#[tokio::test]
async fn a_commit_is_refused_when_head_moved_since_it_was_reviewed() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "reviewed.txt", b"reviewed\n");
    git(&root, &["add", "reviewed.txt"]);
    let action = commit_now(&root, "Add reviewed").await;

    // HEAD moves out from under the reviewed snapshot before the click lands;
    // an empty commit leaves the staged index (and its tree identity)
    // untouched, so HEAD is the only component of the snapshot that moved.
    git(&root, &["commit", "-q", "--allow-empty", "-m", "external"]);
    let moved_head = git_stdout(&root, &["rev-parse", "HEAD"]);

    let error = try_act(&root, action).await.expect_err("HEAD moved");
    assert_eq!(error.code(), "SOURCE_CONTROL_STATE_CHANGED");
    assert_eq!(
        error.message(),
        "the repository changed since it was reviewed: branch, HEAD, index, or merge state"
    );
    assert_eq!(error.effect(), Some("notApplied"));
    assert_eq!(git_stdout(&root, &["rev-parse", "HEAD"]), moved_head);
    assert_eq!(git_stdout(&root, &["log", "-1", "--format=%s"]), "external");
}

/// A `post-commit` hook that corrupts HEAD makes the tree comparison Git
/// commit itself just landed unreadable.
#[cfg(unix)]
#[tokio::test]
async fn landed_commit_with_an_unreadable_head_afterwards_is_not_a_git_error() {
    let (_temp, root) = init_repo_with_commit();
    write_hook(&root, "post-commit", "printf garbage > .git/HEAD\n");
    let before = git_stdout(&root, &["rev-parse", "HEAD"]);
    write(&root, "base.txt", b"changed\n");
    git(&root, &["add", "base.txt"]);

    let action = commit_now(&root, "Break HEAD").await;
    let error = try_act(&root, action).await.expect_err("HEAD unreadable after landing");
    assert_eq!(error.code(), "ACTION_APPLIED_STATUS_UNAVAILABLE");
    assert!(
        error
            .message()
            .starts_with("commit completed but its resulting state could not be read: "),
        "{}",
        error.message()
    );
    assert_eq!(error.effect(), Some("unknown"));

    // The commit landed: with HEAD repaired, main points at it.
    std::fs::write(root.join(".git/HEAD"), b"ref: refs/heads/main\n").expect("repair HEAD");
    assert_ne!(git_stdout(&root, &["rev-parse", "HEAD"]), before);
    assert_eq!(
        git_stdout(&root, &["log", "-1", "--format=%s"]),
        "Break HEAD"
    );
}

/// An unmerged path above a subdirectory root is invisible to this root's
/// lists, but it still blocks the whole-index commit the COMMIT button makes.
/// It is counted in `omitted.unmerged_outside_root` so the UI can say so, and
/// the commit precondition refuses on the repository-wide unmerged flag.
#[tokio::test]
async fn subdirectory_root_counts_unmerged_paths_above_it_and_refuses_the_commit() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "sub/inner.txt", b"inner\n");
    git(&root, &["add", "sub/inner.txt"]);
    git(&root, &["commit", "-qm", "sub"]);
    git(&root, &["checkout", "-q", "-b", "feature"]);
    write(&root, "base.txt", b"feature\n");
    git(&root, &["commit", "-qam", "feature"]);
    git(&root, &["checkout", "-q", "main"]);
    write(&root, "base.txt", b"main\n");
    git(&root, &["commit", "-qam", "main"]);
    assert!(!git_succeeds(&root, &["merge", "-q", "feature"]));

    let sub = root.join("sub");
    let status = status(&sub).await;
    assert!(status.conflicts.is_empty(), "the conflict sits above this root");
    assert_eq!(status.omitted.unmerged_outside_root, 1);
    assert_eq!(status.omitted.staged_outside_root, 0);
    assert!(
        !status.committable,
        "an unmerged path anywhere leaves nothing committable"
    );

    let error = try_act(&sub, commit_for(&status, "merge"))
        .await
        .expect_err("an out-of-root conflict still blocks the whole-index commit");
    assert_eq!(error.code(), "GIT_UNMERGED_PATHS");
    assert_eq!(error.effect(), Some("notApplied"));
}
