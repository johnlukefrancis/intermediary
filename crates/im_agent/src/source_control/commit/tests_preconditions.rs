// Path: crates/im_agent/src/source_control/commit/tests_preconditions.rs
// Description: Real-git tests binding a commit to the reviewed snapshot identity, and row/section ownership

use crate::protocol::{
    SourceControlActionPayload as Action, SourceControlChange::*, SourceControlEntryArea::*,
    SourceControlScope,
};

use crate::source_control::tests_support::*;

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
    assert!(
        outcome.status.snapshot_id.is_empty(),
        "no tree, so no snapshot to bind a commit to"
    );
    // Unmerged is answered before the empty snapshot is: the user has rows to
    // resolve, not a stale review to refresh.
    let error = try_act(&root, commit_for(&outcome.status, "should not commit"))
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

/// A commit carries the identity of the repository the user reviewed. Anything
/// staged after that review is a different commit than the one confirmed.
#[tokio::test]
async fn a_commit_is_refused_when_the_snapshot_moved_since_it_was_reviewed() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "reviewed.txt", b"reviewed\n");
    git(&root, &["add", "reviewed.txt"]);
    let reviewed = status(&root).await;

    // An agent or a terminal stages something else before the click lands.
    write(&root, "outside.txt", b"never confirmed\n");
    git(&root, &["add", "outside.txt"]);

    let error = try_act(&root, commit_for(&reviewed, "Add reviewed"))
        .await
        .expect_err("the snapshot moved");
    assert_eq!(error.code(), "SOURCE_CONTROL_STATE_CHANGED");
    assert_eq!(
        error.message(),
        "the repository changed since it was reviewed: branch, HEAD, index, or merge state"
    );
    assert_eq!(error.effect(), Some("notApplied"));
    assert_eq!(git_stdout(&root, &["log", "-1", "--format=%s"]), "baseline");

    let action = commit_now(&root, "Add both").await;
    act(&root, action).await;
    assert_eq!(git_stdout(&root, &["log", "-1", "--format=%s"]), "Add both");
}

/// A torn status read carries no snapshot at all (`snapshotId: ""`). That
/// empty value must never be *compared* — two empties would agree and
/// authorize a commit of a repository nobody reviewed — so it is refused
/// outright until a clean read replaces it.
#[tokio::test]
async fn a_commit_naming_no_reviewed_snapshot_is_refused() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "reviewed.txt", b"reviewed\n");
    git(&root, &["add", "reviewed.txt"]);
    assert!(
        !status(&root).await.snapshot_id.is_empty(),
        "this read was not torn"
    );

    let error = try_act(
        &root,
        Action::Commit {
            message: "Add reviewed".to_string(),
            expected_snapshot_id: String::new(),
        },
    )
    .await
    .expect_err("no reviewed snapshot");
    assert_eq!(error.code(), "SOURCE_CONTROL_STATE_CHANGED");
    assert_eq!(
        error.message(),
        "the review did not capture a stable snapshot; refresh and retry"
    );
    assert_eq!(error.effect(), Some("notApplied"));
    assert_eq!(git_stdout(&root, &["log", "-1", "--format=%s"]), "baseline");
}

/// Which branch the commit will move is part of what was reviewed. A switch to
/// a new branch at the same commit leaves HEAD and the index identical, so
/// nothing but the snapshot catches it — and it matters: the commit would land
/// on a branch the user never saw.
#[tokio::test]
async fn a_commit_is_refused_when_the_branch_moved_under_an_identical_head_and_index() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "reviewed.txt", b"reviewed\n");
    git(&root, &["add", "reviewed.txt"]);
    let reviewed = status(&root).await;

    git(&root, &["switch", "-q", "-c", "other"]);
    let switched = status(&root).await;
    assert_eq!(switched.head_sha, reviewed.head_sha, "the same commit");
    assert_eq!(
        switched.index_tree_sha, reviewed.index_tree_sha,
        "the same index"
    );
    assert_ne!(switched.snapshot_id, reviewed.snapshot_id);

    let error = try_act(&root, commit_for(&reviewed, "Add reviewed"))
        .await
        .expect_err("the branch moved");
    assert_eq!(error.code(), "SOURCE_CONTROL_STATE_CHANGED");
    assert_eq!(error.effect(), Some("notApplied"));
    assert_eq!(git_stdout(&root, &["log", "-1", "--format=%s"]), "baseline");

    // The same click, re-reviewed on the branch it would actually land on.
    act(&root, commit_for(&switched, "Add reviewed")).await;
    assert_eq!(
        git_stdout(&root, &["rev-parse", "--abbrev-ref", "HEAD"]),
        "other"
    );
}

/// A merge in progress changes what `git commit` records from the very same
/// index — a merge commit with two parents rather than an ordinary one — and
/// Git holds that intent in `MERGE_HEAD`, nowhere the lists can see. Reviewing
/// a merge and committing after it was aborted is a different commit, and is
/// refused.
#[tokio::test]
async fn a_commit_is_refused_when_the_merge_being_concluded_disappeared() {
    let (_temp, root) = init_repo_with_commit();
    git(&root, &["checkout", "-q", "-b", "feature"]);
    git(&root, &["commit", "-q", "--allow-empty", "-m", "feature"]);
    git(&root, &["checkout", "-q", "main"]);
    git(&root, &["merge", "-q", "--no-ff", "--no-commit", "feature"]);

    let merging = status(&root).await;
    assert!(merging.committable, "a merge in progress is committable");
    git(&root, &["merge", "--abort"]);
    let aborted = status(&root).await;
    assert_eq!(aborted.head_sha, merging.head_sha, "the same commit");
    assert_eq!(
        aborted.index_tree_sha, merging.index_tree_sha,
        "the same index"
    );
    assert_ne!(
        aborted.snapshot_id, merging.snapshot_id,
        "MERGE_HEAD is part of the identity"
    );

    let error = try_act(&root, commit_for(&merging, "Merge feature"))
        .await
        .expect_err("the merge was abandoned");
    assert_eq!(error.code(), "SOURCE_CONTROL_STATE_CHANGED");
    assert_eq!(error.effect(), Some("notApplied"));
    assert_eq!(git_stdout(&root, &["log", "-1", "--format=%s"]), "baseline");
}
