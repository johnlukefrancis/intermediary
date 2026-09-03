// Path: crates/im_agent/src/source_control/tests_preconditions.rs
// Description: Real-git tests binding a commit to the reviewed index identity, and row/section ownership

use crate::protocol::{
    SourceControlActionPayload as Action, SourceControlChange::*, SourceControlEntryArea::*,
    SourceControlScope,
};

use super::tests_support::*;

/// Repository whose HEAD holds `source.txt`, with `copy.txt` staged as a copy
/// of it and an unrelated edit waiting in `source.txt`. Git detects the copy
/// because the source is modified in the same staged diff.
fn repo_with_a_staged_copy() -> (tempfile::TempDir, std::path::PathBuf) {
    let (temp, root) = init_repo_with_commit();
    write(&root, "source.txt", b"source\n");
    git(&root, &["add", "source.txt"]);
    git(&root, &["commit", "-qm", "source"]);
    // Copy detection is off by default in status; the row this test is about
    // only exists when it is on.
    git(&root, &["config", "status.renames", "copies"]);
    write(&root, "source.txt", b"source modified\n");
    write(&root, "copy.txt", b"source\n");
    git(&root, &["add", "source.txt", "copy.txt"]);
    write(&root, "source.txt", b"SOURCE EDIT THAT MUST SURVIVE\n");
    (temp, root)
}

/// A copy row names one file. Its provenance is history, not the other half of
/// a move, so discarding the copy may never reach the source file — which the
/// confirmation never mentioned and which holds unrelated work.
#[tokio::test]
async fn discarding_a_copy_leaves_the_source_edit_intact() {
    let (_temp, root) = repo_with_a_staged_copy();
    let before = status(&root).await;
    let copy = before
        .index
        .iter()
        .find(|entry| entry.path == "copy.txt")
        .expect("copy row");
    assert_eq!(copy.change, Copied);
    assert_eq!(copy.original_path.as_deref(), Some("source.txt"));

    act(&root, discard_now(&root, &["copy.txt"])).await;

    assert_eq!(
        read(&root, "source.txt"),
        b"SOURCE EDIT THAT MUST SURVIVE\n",
        "the source file was never a target"
    );
    assert_eq!(git_stdout(&root, &["ls-files", "copy.txt"]), "copy.txt");
}

/// The same rule for staging: the section a row belongs to is the only thing a
/// row action touches.
#[tokio::test]
async fn staging_the_changes_section_never_reaches_a_conflict() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "ordinary.txt", b"ordinary\n");
    git(&root, &["add", "."]);
    git(&root, &["commit", "-qm", "two files"]);
    git(&root, &["checkout", "-q", "-b", "feature"]);
    write(&root, "base.txt", b"feature\n");
    git(&root, &["commit", "-qam", "feature"]);
    git(&root, &["checkout", "-q", "main"]);
    write(&root, "base.txt", b"main\n");
    git(&root, &["commit", "-qam", "main"]);
    assert!(!git_succeeds(&root, &["merge", "-q", "feature"]));
    write(&root, "ordinary.txt", b"edited during the merge\n");

    let outcome = act(
        &root,
        Action::Stage {
            scope: SourceControlScope::All,
        },
    )
    .await;

    assert_eq!(
        stripped(&outcome.status.conflicts),
        vec![entry("base.txt", Conflict, Unmerged)],
        "the conflict stays unmerged"
    );
    assert_eq!(
        outcome.status.index,
        vec![entry("ordinary.txt", Index, Modified)]
    );
    assert!(
        !outcome.status.committable,
        "Git refuses a commit while a path is unmerged"
    );
    assert!(outcome.status.index_tree_sha.is_empty(), "no candidate tree");
    let error = try_act(
        &root,
        Action::Commit {
            message: "should not commit".to_string(),
            expected_index_tree_sha: outcome.status.index_tree_sha.clone(),
            expected_head_sha: outcome.status.head_sha.clone(),
        },
    )
    .await
    .expect_err("unmerged");
    assert_eq!(error.code(), "GIT_UNMERGED_PATHS");
    assert_eq!(error.effect(), Some("notApplied"));
}

#[tokio::test]
async fn the_index_identity_is_the_tree_git_would_write() {
    let (_temp, root) = init_repo_with_commit();
    assert_eq!(
        status(&root).await.index_tree_sha,
        git_stdout(&root, &["write-tree"])
    );
    write(&root, "sub/added.txt", b"added\n");
    git(&root, &["add", "."]);
    let staged = status(&root).await;
    assert_eq!(staged.index_tree_sha, git_stdout(&root, &["write-tree"]));
    // A subdirectory root commits the whole index, so it reports the same
    // identity as the top level.
    assert_eq!(
        status(&root.join("sub")).await.index_tree_sha,
        staged.index_tree_sha
    );
}

/// A commit carries the identity of the index the user reviewed. Anything
/// staged after that review is a different commit than the one confirmed.
#[tokio::test]
async fn a_commit_is_refused_when_the_index_moved_since_it_was_reviewed() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "reviewed.txt", b"reviewed\n");
    git(&root, &["add", "reviewed.txt"]);
    let reviewed = status(&root).await;

    // An agent or a terminal stages something else before the click lands.
    write(&root, "outside.txt", b"never confirmed\n");
    git(&root, &["add", "outside.txt"]);

    let error = try_act(
        &root,
        Action::Commit {
            message: "Add reviewed".to_string(),
            expected_index_tree_sha: reviewed.index_tree_sha,
            expected_head_sha: reviewed.head_sha,
        },
    )
    .await
    .expect_err("index moved");
    assert_eq!(error.code(), "SOURCE_CONTROL_STATE_CHANGED");
    assert_eq!(error.message(), "index changed since it was reviewed");
    assert_eq!(error.effect(), Some("notApplied"));
    assert_eq!(git_stdout(&root, &["log", "-1", "--format=%s"]), "baseline");

    let action = commit_now(&root, "Add both").await;
    act(&root, action).await;
    assert_eq!(git_stdout(&root, &["log", "-1", "--format=%s"]), "Add both");
}
