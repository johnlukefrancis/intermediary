// Path: crates/im_agent/src/source_control/tests_commit.rs
// Description: Real-git tempdir tests for the commit oracle, commit outcomes, and the landed-but-unread error

use crate::protocol::{
    SourceControlActionPayload as Action, SourceControlChange::*, SourceControlEntryArea::*,
};

use super::tests_support::*;

#[tokio::test]
async fn commit_returns_the_new_head_and_clears_the_index() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "base.txt", b"changed\n");
    git(&root, &["add", "base.txt"]);
    let outcome = act(
        &root,
        Action::Commit {
            message: "  Change base\n\n".to_string(),
        },
    )
    .await;
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
    let error = try_act(
        &root,
        Action::Commit {
            message: "message".to_string(),
        },
    )
    .await
    .expect_err("nothing staged");
    assert_eq!(error.code(), "GIT_NOTHING_TO_COMMIT");

    git(&root, &["add", "base.txt"]);
    let error = try_act(
        &root,
        Action::Commit {
            message: "  \n".to_string(),
        },
    )
    .await
    .expect_err("blank message");
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

    let outcome = act(
        &root,
        Action::Commit {
            message: "Merge feature".to_string(),
        },
    )
    .await;
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

/// A `post-commit` hook that corrupts HEAD makes the follow-up `rev-parse`
/// fail after the commit itself has landed.
#[cfg(unix)]
#[tokio::test]
async fn landed_commit_with_a_failing_follow_up_read_is_not_a_git_error() {
    use std::os::unix::fs::PermissionsExt;

    let (_temp, root) = init_repo_with_commit();
    let hook = root.join(".git/hooks/post-commit");
    std::fs::create_dir_all(hook.parent().expect("hooks dir")).expect("hooks dir");
    std::fs::write(&hook, "#!/bin/sh\nprintf garbage > .git/HEAD\n").expect("hook");
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).expect("hook mode");
    let before = git_stdout(&root, &["rev-parse", "HEAD"]);
    write(&root, "base.txt", b"changed\n");
    git(&root, &["add", "base.txt"]);

    let error = try_act(
        &root,
        Action::Commit {
            message: "Break HEAD".to_string(),
        },
    )
    .await
    .expect_err("follow-up read fails");
    assert_eq!(error.code(), "ACTION_APPLIED_STATUS_UNAVAILABLE");
    assert!(
        error
            .message()
            .starts_with("commit completed but the follow-up status read failed: "),
        "{}",
        error.message()
    );
    assert_eq!(
        error.details(),
        Some(&serde_json::json!({ "kind": "commit", "commitSha": null }))
    );

    // The commit landed: with HEAD repaired, main points at it.
    std::fs::write(root.join(".git/HEAD"), b"ref: refs/heads/main\n").expect("repair HEAD");
    assert_ne!(git_stdout(&root, &["rev-parse", "HEAD"]), before);
    assert_eq!(
        git_stdout(&root, &["log", "-1", "--format=%s"]),
        "Break HEAD"
    );
}
