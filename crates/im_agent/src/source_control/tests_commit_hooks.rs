// Path: crates/im_agent/src/source_control/tests_commit_hooks.rs
// Description: Real-git tests for what a commit hook may change: the reviewed set is accepted, anything beyond it is retracted

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;

use crate::protocol::SourceControlActionPayload as Action;

use super::tests_support::*;

/// Installs an executable `pre-commit` hook whose body Git runs from the
/// worktree's top level, whatever root the action itself was sent to.
fn write_pre_commit_hook(root: &std::path::Path, body: &str) {
    let hook = root.join(".git/hooks/pre-commit");
    std::fs::create_dir_all(hook.parent().expect("hooks dir")).expect("hooks dir");
    std::fs::write(&hook, format!("#!/bin/sh\n{body}")).expect("hook");
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).expect("hook mode");
}

/// A repository whose configured root is the `sub/` subdirectory, with one
/// in-root and one outside-root path staged: the shape in which the UI shows
/// its outside-root warning and the user acknowledges it before committing.
fn repo_with_an_acknowledged_outside_root_path() -> (tempfile::TempDir, std::path::PathBuf) {
    let (temp, root) = init_repo_with_commit();
    write(&root, "sub/reviewed.txt", b"reviewed\n");
    write(&root, "outside.txt", b"outside\n");
    git(&root, &["add", "."]);
    git(&root, &["commit", "-qm", "add sub and outside"]);

    write(&root, "sub/reviewed.txt", b"reviewed again\n");
    write(&root, "outside.txt", b"outside again\n");
    git(&root, &["add", "."]);
    (temp, root)
}

/// A `pre-commit` hook that reformats an already-reviewed file and re-stages
/// it is accepted: the change lands inside the same commit, reported through
/// `hookChangedPaths`.
#[tokio::test]
async fn a_hook_that_reformats_a_reviewed_path_is_applied_with_hook_changed_paths() {
    let (_temp, root) = init_repo_with_commit();
    write_pre_commit_hook(
        &root,
        "printf 'reformatted\\n' >> reviewed.txt\ngit add -- reviewed.txt\n",
    );
    write(&root, "reviewed.txt", b"before\n");
    git(&root, &["add", "reviewed.txt"]);

    let action = commit_now(&root, "Add reviewed").await;
    let outcome = act(&root, action).await;
    assert_eq!(outcome.hook_changed_paths, vec!["reviewed.txt".to_string()]);
    let head = git_stdout(&root, &["rev-parse", "HEAD"]);
    assert_eq!(outcome.commit_sha.as_deref(), Some(head.as_str()));
    assert_eq!(read(&root, "reviewed.txt"), b"before\nreformatted\n");
    assert_eq!(git_stdout(&root, &["log", "-1", "--format=%s"]), "Add reviewed");
}

/// A `pre-commit` hook that stages a path the user never reviewed, and which
/// sits outside the configured root, is retracted: HEAD is restored to
/// exactly where it was, and the hook's own `git add` is left in the index
/// for the user to see and review, not silently discarded.
#[tokio::test]
async fn a_hook_that_stages_an_unreviewed_outside_root_path_is_retracted() {
    let (_temp, root) = init_repo_with_commit();
    std::fs::create_dir_all(root.join("sub")).expect("sub dir");
    write(&root, "sub/reviewed.txt", b"reviewed\n");
    git(&root, &["add", "sub"]);
    git(&root, &["commit", "-qm", "add sub"]);

    write_pre_commit_hook(&root, "printf 'unreviewed\\n' > outside.txt\ngit add -- outside.txt\n");

    let sub = root.join("sub");
    write(&sub, "reviewed.txt", b"reviewed again\n");
    git(&sub, &["add", "reviewed.txt"]);
    let before_head = git_stdout(&root, &["rev-parse", "HEAD"]);
    let reviewed = status(&sub).await;
    assert_eq!(reviewed.omitted.staged_outside_root, 0, "outside.txt is not staged yet");

    let error = try_act(
        &sub,
        Action::Commit {
            message: "Update reviewed".to_string(),
            expected_index_tree_sha: reviewed.index_tree_sha,
            expected_head_sha: reviewed.head_sha,
        },
    )
    .await
    .expect_err("hook overreached");
    assert_eq!(error.code(), "SOURCE_CONTROL_STATE_CHANGED");
    assert!(
        error.message().starts_with("a commit hook staged unreviewed paths: "),
        "{}",
        error.message()
    );
    assert!(error.message().contains("outside.txt"), "{}", error.message());
    assert_eq!(error.effect(), Some("notApplied"));

    assert_eq!(
        git_stdout(&root, &["rev-parse", "HEAD"]),
        before_head,
        "HEAD is retracted to exactly where it was"
    );
    assert_eq!(
        git_stdout(&root, &["diff", "--cached", "--name-only"]),
        "outside.txt\nsub/reviewed.txt",
        "the hook's own staging survives for the user to review"
    );
}

/// The outside-root path the user acknowledged is part of what they
/// reviewed, so a hook that reformats and re-stages it is accepted exactly
/// like an in-root reformat. The acknowledged set is read from the reviewed
/// index tree: the live index this once read is empty by now — the commit
/// consumed it — so reading it there would retract a commit the user
/// confirmed.
#[tokio::test]
async fn a_hook_that_reformats_an_acknowledged_outside_root_path_is_applied() {
    let (_temp, root) = repo_with_an_acknowledged_outside_root_path();
    write_pre_commit_hook(
        &root,
        "printf 'reformatted\\n' >> outside.txt\ngit add -- outside.txt\n",
    );

    let sub = root.join("sub");
    let reviewed = status(&sub).await;
    assert_eq!(
        reviewed.omitted.staged_outside_root, 1,
        "outside.txt is staged, and the UI asked before sending"
    );

    let outcome = act(
        &sub,
        Action::Commit {
            message: "Update reviewed".to_string(),
            expected_index_tree_sha: reviewed.index_tree_sha,
            expected_head_sha: reviewed.head_sha,
        },
    )
    .await;

    assert_eq!(outcome.hook_changed_paths, vec!["outside.txt".to_string()]);
    let head = git_stdout(&root, &["rev-parse", "HEAD"]);
    assert_eq!(outcome.commit_sha.as_deref(), Some(head.as_str()));
    assert_eq!(read(&root, "outside.txt"), b"outside again\nreformatted\n");
    assert_eq!(
        git_stdout(&root, &["log", "-1", "--format=%s"]),
        "Update reviewed"
    );
}

/// Acknowledging one outside-root path licenses that path, not the rest of
/// the repository: a hook that stages a second outside path nobody reviewed
/// is still retracted.
#[tokio::test]
async fn a_hook_that_stages_a_second_unreviewed_outside_path_is_still_retracted() {
    let (_temp, root) = repo_with_an_acknowledged_outside_root_path();
    write_pre_commit_hook(&root, "printf 'sneaky\\n' > sneaky.txt\ngit add -- sneaky.txt\n");

    let sub = root.join("sub");
    let before_head = git_stdout(&root, &["rev-parse", "HEAD"]);
    let reviewed = status(&sub).await;
    assert_eq!(reviewed.omitted.staged_outside_root, 1);

    let error = try_act(
        &sub,
        Action::Commit {
            message: "Update reviewed".to_string(),
            expected_index_tree_sha: reviewed.index_tree_sha,
            expected_head_sha: reviewed.head_sha,
        },
    )
    .await
    .expect_err("hook overreached");
    assert_eq!(error.code(), "SOURCE_CONTROL_STATE_CHANGED");
    assert!(error.message().contains("sneaky.txt"), "{}", error.message());
    assert_eq!(error.effect(), Some("notApplied"));
    assert_eq!(
        git_stdout(&root, &["rev-parse", "HEAD"]),
        before_head,
        "HEAD is retracted to exactly where it was"
    );
    assert_eq!(read(&root, "outside.txt"), b"outside again\n");
}
