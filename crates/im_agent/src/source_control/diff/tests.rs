// Path: crates/im_agent/src/source_control/diff/tests.rs
// Description: Real-git tempdir tests for bounded per-file diff capture

use crate::protocol::SourceControlArea;

use crate::source_control::source_control_diff;
use crate::source_control::tests_support::*;

#[tokio::test]
async fn diff_covers_index_worktree_untracked_and_renames() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "base.txt", b"staged\n");
    git(&root, &["add", "base.txt"]);
    write(&root, "base.txt", b"staged then edited\n");
    write(&root, "new.txt", b"hello untracked\n");

    let index = source_control_diff(&root, "base.txt", None, SourceControlArea::Index, None)
        .await
        .expect("index diff");
    assert!(index.patch.contains("-base\n"));
    assert!(index.patch.contains("+staged\n"));
    assert!(!index.patch.contains("staged then edited"));
    assert!(!index.binary && !index.truncated);

    let worktree = source_control_diff(&root, "base.txt", None, SourceControlArea::Worktree, None)
        .await
        .expect("worktree diff");
    assert!(worktree.patch.contains("-staged\n"));
    assert!(worktree.patch.contains("+staged then edited\n"));

    let untracked = source_control_diff(&root, "new.txt", None, SourceControlArea::Worktree, None)
        .await
        .expect("untracked diff");
    assert!(untracked.patch.contains("--- /dev/null"));
    assert!(untracked.patch.contains("+hello untracked\n"));
    assert!(!untracked.binary);

    git(&root, &["add", "new.txt"]);
    git(&root, &["commit", "-qm", "add new"]);
    git(&root, &["mv", "new.txt", "moved.txt"]);
    let renamed = source_control_diff(
        &root,
        "moved.txt",
        Some("new.txt"),
        SourceControlArea::Index,
        None,
    )
    .await
    .expect("rename diff");
    assert!(renamed.patch.contains("rename from new.txt"));
    assert!(renamed.patch.contains("rename to moved.txt"));
}

#[tokio::test]
async fn diff_flags_binary_files() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "blob.bin", b"\0\x01\x02");
    git(&root, &["add", "blob.bin"]);
    git(&root, &["commit", "-qm", "binary"]);
    write(&root, "blob.bin", b"\0\x03\x04\x05");

    let diff = source_control_diff(&root, "blob.bin", None, SourceControlArea::Worktree, None)
        .await
        .expect("binary diff");
    assert!(diff.binary);
    assert!(diff.patch.contains("Binary files"));
    assert!(!diff.truncated);
}

#[tokio::test]
async fn diff_rejects_invalid_paths_before_running_git() {
    let (_temp, root) = init_repo_with_commit();
    let error = source_control_diff(
        &root,
        "../base.txt",
        None,
        SourceControlArea::Worktree,
        None,
    )
    .await
    .expect_err("traversal rejected");
    assert_eq!(error.code(), "INVALID_PATH");
}

#[tokio::test]
async fn diff_headers_keep_non_ascii_names_unescaped() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "héllo.txt", b"hi\n");
    git(&root, &["add", "héllo.txt"]);
    write(&root, "ünstaged.txt", b"loose\n");

    let index = source_control_diff(&root, "héllo.txt", None, SourceControlArea::Index, None)
        .await
        .expect("index diff");
    assert!(index.patch.contains("+++ b/héllo.txt"), "{}", index.patch);
    assert!(
        !index.patch.contains("\\303\\251"),
        "octal-escaped name: {}",
        index.patch
    );

    let untracked = source_control_diff(
        &root,
        "ünstaged.txt",
        None,
        SourceControlArea::Worktree,
        None,
    )
    .await
    .expect("untracked diff");
    assert!(
        untracked.patch.contains("+++ b/ünstaged.txt"),
        "{}",
        untracked.patch
    );
}
