// Path: crates/im_agent/src/source_control/tests.rs
// Description: Real-git tempdir tests for source-control status projection, the commit oracle, and error mapping

use im_bundle::git::GitCommandOutput;

use crate::protocol::{SourceControlChange::*, SourceControlEntryArea::*, SourceControlOmitted};

use super::source_control_status;
use super::status_project::project_status;
use super::SourceControlLocks;
use super::tests_support::*;

#[tokio::test]
async fn status_splits_a_path_staged_and_modified_again() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "base.txt", b"staged\n");
    git(&root, &["add", "base.txt"]);
    write(&root, "base.txt", b"staged then edited\n");

    let status = status(&root).await;
    assert_eq!(status.branch.as_deref(), Some("main"));
    assert!(status.head_sha.is_some());
    assert!(!status.detached);
    assert_eq!(status.upstream, None);
    assert_eq!((status.ahead, status.behind), (None, None));
    assert_eq!(status.index, vec![entry("base.txt", Index, Modified)]);
    assert_eq!(stripped(&status.worktree), vec![entry("base.txt", Worktree, Modified)]);
    assert!(status.conflicts.is_empty());
    assert_eq!(status.omitted, SourceControlOmitted::default());
    assert!(status.committable);
    assert!(!status.truncated);
    assert!(status.captured_at_iso.ends_with('Z'));
}

#[tokio::test]
async fn status_maps_added_deleted_renamed_and_untracked() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "old.txt", b"rename me\n");
    write(&root, "gone.txt", b"delete me\n");
    git(&root, &["add", "."]);
    git(&root, &["commit", "-qm", "second"]);
    write(&root, "new.txt", b"new\n");
    git(&root, &["add", "new.txt"]);
    git(&root, &["rm", "-q", "gone.txt"]);
    git(&root, &["mv", "old.txt", "moved.txt"]);
    write(&root, "untracked.txt", b"loose\n");

    let status = status(&root).await;
    assert_eq!(
        status.index,
        vec![
            entry("gone.txt", Index, Deleted),
            renamed_entry("moved.txt", "old.txt"),
            entry("new.txt", Index, Added),
        ]
    );
    assert_eq!(
        stripped(&status.worktree),
        vec![entry("untracked.txt", Worktree, Untracked)]
    );
}

#[tokio::test]
async fn status_lists_conflicts_after_a_conflicting_merge() {
    let (_temp, root) = init_repo_with_commit();
    git(&root, &["checkout", "-q", "-b", "feature"]);
    write(&root, "base.txt", b"feature\n");
    git(&root, &["commit", "-qam", "feature"]);
    git(&root, &["checkout", "-q", "main"]);
    write(&root, "base.txt", b"main\n");
    git(&root, &["commit", "-qam", "main"]);
    assert!(!git_succeeds(&root, &["merge", "-q", "feature"]));

    let status = status(&root).await;
    assert_eq!(stripped(&status.conflicts), vec![entry("base.txt", Conflict, Unmerged)]);
    assert!(status.index.is_empty());
    assert!(status.worktree.is_empty());
}

#[tokio::test]
async fn status_reports_detached_head() {
    let (_temp, root) = init_repo_with_commit();
    git(&root, &["checkout", "-q", "--detach"]);
    let status = status(&root).await;
    assert_eq!(status.branch, None);
    assert!(status.detached);
    assert_eq!(status.head_sha.as_deref(), Some(git_stdout(&root, &["rev-parse", "HEAD"]).as_str()));
}

#[tokio::test]
async fn status_on_an_unborn_branch_has_a_branch_but_no_head() {
    let (_temp, root) = init_repo();
    write(&root, "a.txt", b"a\n");
    git(&root, &["add", "a.txt"]);
    let status = status(&root).await;
    assert_eq!(status.branch.as_deref(), Some("main"));
    assert_eq!(status.head_sha, None);
    assert!(!status.detached);
    assert_eq!(status.index, vec![entry("a.txt", Index, Added)]);
}

#[tokio::test]
async fn subdirectory_root_strips_the_prefix_and_counts_staged_outside_paths() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "sub/inner.txt", b"inner\n");
    write(&root, "top.txt", b"top\n");
    write(&root, "base.txt", b"changed outside\n");
    git(&root, &["add", "top.txt"]);

    let status = status(&root.join("sub")).await;
    assert_eq!(stripped(&status.worktree), vec![entry("inner.txt", Worktree, Untracked)]);
    assert!(status.index.is_empty());
    assert_eq!(
        status.omitted.staged_outside_root, 1,
        "top.txt is staged; the worktree-only base.txt is dropped uncounted"
    );
    assert_eq!(status.omitted.unrepresentable_path, 0);
    assert!(status.committable, "the whole index commits, staged-outside paths included");
}

/// A rename whose source is outside the configured root is still one record:
/// the entry belongs here, its outside endpoint cannot be named here, and the
/// deletion that travels with the same commit is counted so the user is warned.
#[tokio::test]
async fn a_rename_into_the_root_counts_the_outside_deletion_it_carries() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "sub/keep.txt", b"keep\n");
    write(&root, "outside.txt", b"moving\n");
    git(&root, &["add", "."]);
    git(&root, &["commit", "-qm", "before the move"]);
    git(&root, &["mv", "outside.txt", "sub/inside.txt"]);

    let status = status(&root.join("sub")).await;
    assert_eq!(
        status.index,
        vec![entry("inside.txt", Index, Renamed)],
        "the outside source cannot be named from this root"
    );
    assert_eq!(status.omitted.staged_outside_root, 1);
}

/// The mirror case: the visible file leaves the root. The deletion inside the
/// root is what this root can show, and the new outside path is counted.
#[tokio::test]
async fn a_rename_out_of_the_root_lists_the_deletion_it_leaves_behind() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "sub/inside.txt", b"moving\n");
    git(&root, &["add", "."]);
    git(&root, &["commit", "-qm", "before the move"]);
    git(&root, &["mv", "sub/inside.txt", "outside.txt"]);

    let status = status(&root.join("sub")).await;
    assert_eq!(status.index, vec![entry("inside.txt", Index, Deleted)]);
    assert!(status.worktree.is_empty());
    assert_eq!(status.omitted.staged_outside_root, 1);
}

#[tokio::test]
async fn committable_follows_git_not_the_projected_index_list() {
    let (_repo, root) = init_repo_with_commit();
    assert!(!status(&root).await.committable, "clean repo");
    write(&root, "base.txt", b"unstaged\n");
    assert!(!status(&root).await.committable, "worktree-only change");
    git(&root, &["add", "base.txt"]);
    assert!(status(&root).await.committable, "staged change");

    let (_unborn, root) = init_repo();
    assert!(!status(&root).await.committable, "empty unborn branch");
    write(&root, "a.txt", b"a\n");
    git(&root, &["add", "a.txt"]);
    assert!(
        status(&root).await.committable,
        "an unborn branch compares the index against the empty tree"
    );
}

#[test]
fn truncated_status_is_cut_at_the_last_nul_and_flagged() {
    let stdout = b"# branch.oid abc\0# branch.head main\0\
1 M. N... 100644 100644 100644 0 0 a.txt\0\
1 .M N... 100644 100644 100644 0 0 b.txt\0\
2 R. N... 100644 100644 100644 0 0 R100 renamed.txt\0"
        .to_vec();
    let status = project_status(
        b"",
        GitCommandOutput {
            stdout,
            stdout_truncated: true,
            stderr: Vec::new(),
            exit_code: 0,
        },
        false,
        String::new(),
        false,
    )
    .expect("best-effort projection")
    .status;
    assert!(status.truncated);
    assert_eq!(paths_of(&status.index), vec!["a.txt", "renamed.txt"]);
    assert_eq!(status.index[1].original_path, None, "torn rename source is dropped");
    assert_eq!(paths_of(&status.worktree), vec!["b.txt"]);
    assert_eq!(status.head_sha.as_deref(), Some("abc"));
}

#[tokio::test]
async fn non_repository_directory_is_reported() {
    let temp = tempfile::tempdir().expect("tempdir");
    let error = source_control_status(temp.path(), None, &SourceControlLocks::new())
        .await
        .expect_err("not a repository");
    assert_eq!(error.code(), "GIT_NOT_REPOSITORY");
}

#[tokio::test]
async fn missing_root_is_invalid_repo() {
    let temp = tempfile::tempdir().expect("tempdir");
    let missing = temp.path().join("gone");
    let error = source_control_status(&missing, None, &SourceControlLocks::new())
        .await
        .expect_err("missing root");
    assert_eq!(error.code(), "INVALID_REPO");
    assert!(error.message().contains("Repo root no longer exists"));
}
