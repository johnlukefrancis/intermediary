// Path: crates/im_agent/src/source_control/tests_actions.rs
// Description: Real-git tempdir tests for stage, unstage, discard, push, and pull actions

use crate::protocol::{
    SourceControlActionPayload as Action, SourceControlChange::*, SourceControlEntryArea::*,
    SourceControlScope,
};

use super::tests_support::*;

#[tokio::test]
async fn stage_paths_with_glob_and_magic_characters_literally() {
    let (_temp, root) = init_repo();
    for name in ["a[1].txt", "star*.txt", ":colon.txt", "plain.txt"] {
        write(&root, name, b"x\n");
    }
    let outcome = act(
        &root,
        Action::Stage {
            scope: paths_scope(&["a[1].txt", "star*.txt", ":colon.txt"]),
        },
    )
    .await;
    assert_eq!(
        outcome.status.index,
        vec![
            entry(":colon.txt", Index, Added),
            entry("a[1].txt", Index, Added),
            entry("star*.txt", Index, Added),
        ]
    );
    assert_eq!(
        stripped(&outcome.status.worktree),
        vec![entry("plain.txt", Worktree, Untracked)]
    );
    assert_eq!(outcome.commit_sha, None);
}

/// An empty list is a mistake the UI must not be able to make silently: to Git,
/// zero pathspecs mean the whole repository.
#[tokio::test]
async fn empty_path_lists_are_refused_before_git_runs() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "base.txt", b"changed\n");
    git(&root, &["add", "base.txt"]);
    for action in [
        Action::Unstage {
            scope: paths_scope(&[]),
        },
        Action::Stage {
            scope: paths_scope(&[]),
        },
        Action::Discard {
            targets: Vec::new(),
        },
    ] {
        let error = try_act(&root, action).await.expect_err("empty list");
        assert_eq!(error.code(), "INVALID_PATH");
        assert_eq!(error.message(), "No paths given");
        assert_eq!(error.effect(), Some("notApplied"));
    }
    assert_eq!(
        status(&root).await.index,
        vec![entry("base.txt", Index, Modified)]
    );
}

#[tokio::test]
async fn stage_all_records_a_deletion() {
    let (_temp, root) = init_repo_with_commit();
    std::fs::remove_file(root.join("base.txt")).expect("delete tracked file");
    assert_eq!(
        stripped(&status(&root).await.worktree),
        vec![entry("base.txt", Worktree, Deleted)]
    );
    let outcome = act(
        &root,
        Action::Stage {
            scope: SourceControlScope::All,
        },
    )
    .await;
    assert_eq!(outcome.status.index, vec![entry("base.txt", Index, Deleted)]);
    assert!(outcome.status.worktree.is_empty());
}

#[tokio::test]
async fn unstage_works_on_an_unborn_branch() {
    let (_temp, root) = init_repo();
    write(&root, "a.txt", b"a\n");
    write(&root, "b.txt", b"b\n");
    git(&root, &["add", "."]);
    let outcome = act(
        &root,
        Action::Unstage {
            scope: paths_scope(&["a.txt"]),
        },
    )
    .await;
    assert_eq!(outcome.status.index, vec![entry("b.txt", Index, Added)]);
    assert_eq!(
        stripped(&outcome.status.worktree),
        vec![entry("a.txt", Worktree, Untracked)]
    );
    let outcome = act(
        &root,
        Action::Unstage {
            scope: SourceControlScope::All,
        },
    )
    .await;
    assert!(outcome.status.index.is_empty());
    assert_eq!(paths_of(&outcome.status.worktree), vec!["a.txt", "b.txt"]);
}

#[tokio::test]
async fn discard_restores_tracked_removes_untracked_files_and_skips_unlisted_paths() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "base.txt", b"edited\n");
    write(&root, "junk.txt", b"junk\n");
    write(&root, "dir/inner.txt", b"inner\n");
    let outcome = act(&root, discard_now(&root, &["base.txt", "junk.txt"])).await;
    assert_eq!(read(&root, "base.txt"), b"base\n");
    assert!(!root.join("junk.txt").exists());
    assert_eq!(
        stripped(&outcome.status.worktree),
        vec![entry("dir/inner.txt", Worktree, Untracked)]
    );

    // A directory is never a status entry, so discarding it is a validated no-op.
    let outcome = act(&root, discard_now(&root, &["dir"])).await;
    assert!(root.join("dir/inner.txt").exists());
    assert_eq!(
        stripped(&outcome.status.worktree),
        vec![entry("dir/inner.txt", Worktree, Untracked)]
    );
}

#[tokio::test]
async fn discard_of_an_intent_to_add_file_removes_it_and_its_index_entry() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "new.txt", b"new\n");
    git(&root, &["add", "-N", "new.txt"]);
    assert_eq!(
        stripped(&status(&root).await.worktree),
        vec![entry("new.txt", Worktree, Added)]
    );
    let outcome = act(&root, discard_now(&root, &["new.txt"])).await;
    assert!(!root.join("new.txt").exists());
    assert!(outcome.status.index.is_empty());
    assert!(outcome.status.worktree.is_empty());
    assert_eq!(git_stdout(&root, &["ls-files", "new.txt"]), "");
}

#[tokio::test]
async fn push_and_pull_track_upstream_ahead_and_behind() {
    let (temp, root) = init_repo_with_commit();
    let remote = temp.path().join("remote.git");
    git(temp.path(), &["init", "-q", "--bare", "-b", "main", "remote.git"]);
    git(&root, &["remote", "add", "origin", remote.to_str().expect("utf8 path")]);
    assert_eq!(status(&root).await.upstream, None);

    let outcome = act(&root, Action::Push).await;
    assert_eq!(outcome.status.upstream.as_deref(), Some("origin/main"));
    assert_eq!((outcome.status.ahead, outcome.status.behind), (Some(0), Some(0)));

    let clone = temp.path().join("clone");
    git(temp.path(), &["clone", "-q", "remote.git", "clone"]);
    write(&clone, "remote.txt", b"from clone\n");
    git(&clone, &["add", "remote.txt"]);
    git(
        &clone,
        &["-c", "user.email=c@example.test", "-c", "user.name=C", "commit", "-qm", "remote change"],
    );
    git(&clone, &["push", "-q"]);
    git(&root, &["fetch", "-q"]);
    assert_eq!(status(&root).await.behind, Some(1));

    let outcome = act(&root, Action::Pull).await;
    assert_eq!((outcome.status.ahead, outcome.status.behind), (Some(0), Some(0)));
    assert_eq!(read(&root, "remote.txt"), b"from clone\n");

    write(&root, "base.txt", b"ahead\n");
    git(&root, &["commit", "-qam", "ahead"]);
    assert_eq!(status(&root).await.ahead, Some(1));
    let outcome = act(&root, Action::Push).await;
    assert_eq!(outcome.status.ahead, Some(0));
}

#[tokio::test]
async fn push_without_any_remote_is_refused() {
    let (_temp, root) = init_repo_with_commit();
    let error = try_act(&root, Action::Push).await.expect_err("no remote");
    assert_eq!(error.code(), "GIT_COMMAND_FAILED");
    assert!(error.message().contains("No upstream"));
}

#[tokio::test]
async fn invalid_paths_are_rejected_before_git_runs() {
    let (_temp, root) = init_repo_with_commit();
    let error = try_act(
        &root,
        Action::Stage {
            scope: paths_scope(&["../outside.txt"]),
        },
    )
    .await
    .expect_err("traversal");
    assert_eq!(error.code(), "INVALID_PATH");
    let error = try_act(&root, discard_now(&root, &["/etc/hosts"]))
        .await
        .expect_err("absolute");
    assert_eq!(error.code(), "INVALID_PATH");
}
