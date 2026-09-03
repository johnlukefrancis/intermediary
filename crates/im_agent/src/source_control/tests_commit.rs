// Path: crates/im_agent/src/source_control/tests_commit.rs
// Description: Real-git tests for the commit oracle, its index/HEAD preconditions, and the landed-but-unread error

use crate::protocol::{SourceControlActionPayload as Action, SourceControlChange::*, SourceControlEntryArea::*};

use super::tests_support::*;

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

/// The reviewed HEAD is a precondition too: a commit that landed on a
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
    // untouched, so only the HEAD precondition — not the index one — is what
    // catches this.
    git(&root, &["commit", "-q", "--allow-empty", "-m", "external"]);
    let moved_head = git_stdout(&root, &["rev-parse", "HEAD"]);

    let error = try_act(&root, action).await.expect_err("HEAD moved");
    assert_eq!(error.code(), "SOURCE_CONTROL_STATE_CHANGED");
    assert_eq!(error.message(), "HEAD changed since it was reviewed");
    assert_eq!(error.effect(), Some("notApplied"));
    assert_eq!(git_stdout(&root, &["rev-parse", "HEAD"]), moved_head);
    assert_eq!(git_stdout(&root, &["log", "-1", "--format=%s"]), "external");
}

/// A `post-commit` hook that corrupts HEAD makes the tree comparison Git
/// commit itself just landed unreadable.
#[cfg(unix)]
#[tokio::test]
async fn landed_commit_with_an_unreadable_head_afterwards_is_not_a_git_error() {
    use std::os::unix::fs::PermissionsExt;

    let (_temp, root) = init_repo_with_commit();
    let hook = root.join(".git/hooks/post-commit");
    std::fs::create_dir_all(hook.parent().expect("hooks dir")).expect("hooks dir");
    std::fs::write(&hook, "#!/bin/sh\nprintf garbage > .git/HEAD\n").expect("hook");
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).expect("hook mode");
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

/// A torn status read reports no index identity at all (`indexTreeSha: ""`).
/// That empty value must never be *compared* — two empties would agree and
/// authorize a commit of a tree nobody reviewed — so it is refused outright
/// until a clean read replaces it.
#[tokio::test]
async fn a_commit_naming_no_reviewed_index_identity_is_refused() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "reviewed.txt", b"reviewed\n");
    git(&root, &["add", "reviewed.txt"]);
    let reviewed = status(&root).await;
    assert!(!reviewed.index_tree_sha.is_empty(), "this read was not torn");

    let error = try_act(
        &root,
        Action::Commit {
            message: "Add reviewed".to_string(),
            expected_index_tree_sha: String::new(),
            expected_head_sha: reviewed.head_sha,
        },
    )
    .await
    .expect_err("no reviewed identity");
    assert_eq!(error.code(), "SOURCE_CONTROL_STATE_CHANGED");
    assert!(
        error.message().starts_with("the reviewed index had no stable identity"),
        "{}",
        error.message()
    );
    assert_eq!(error.effect(), Some("notApplied"));
    assert_eq!(git_stdout(&root, &["log", "-1", "--format=%s"]), "baseline");
}
