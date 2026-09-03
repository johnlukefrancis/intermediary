// Path: crates/im_agent/src/source_control/locks/tests.rs
// Description: Real-git tests for mutation serialization by physical git dir, drain, and mutationInProgress

use std::time::Duration;

use crate::protocol::{SourceControlActionPayload as Action, SourceControlScope};

use crate::source_control::tests_support::*;
use crate::source_control::SourceControlLocks;

const BRIEF: Duration = Duration::from_millis(150);

/// A configured root and a configured subdirectory of it are two UI entries
/// over one index; they must not mutate it at the same time.
#[tokio::test]
async fn a_root_and_a_subdirectory_below_it_share_one_lock() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "sub/inner.txt", b"inner\n");
    git(&root, &["add", "."]);
    git(&root, &["commit", "-qm", "sub"]);
    let locks = SourceControlLocks::new();

    let guard = locks.acquire(&root).await.expect("root lock");
    assert!(
        tokio::time::timeout(BRIEF, locks.acquire(&root.join("sub")))
            .await
            .is_err(),
        "the subdirectory waits for the one index"
    );
    drop(guard);
    locks
        .acquire(&root.join("sub"))
        .await
        .expect("released lock");
}

/// A linked worktree has its own index, so it must not queue behind the
/// primary worktree's mutations.
#[tokio::test]
async fn a_linked_worktree_keeps_its_own_lock() {
    let (temp, root) = init_repo_with_commit();
    let linked = temp.path().join("linked");
    git(
        &root,
        &["worktree", "add", "-q", linked.to_str().expect("utf8"), "-b", "linked"],
    );
    let locks = SourceControlLocks::new();

    let _guard = locks.acquire(&root).await.expect("root lock");
    tokio::time::timeout(BRIEF, locks.acquire(&linked))
        .await
        .expect("the linked worktree does not wait")
        .expect("linked lock");
}

#[tokio::test]
async fn status_reports_a_mutation_in_progress_while_the_lock_is_held() {
    let (_temp, root) = init_repo_with_commit();
    let locks = SourceControlLocks::new();
    assert!(!status_with(&locks, &root).await.mutation_in_progress);

    let guard = locks.acquire(&root).await.expect("lock");
    assert!(status_with(&locks, &root).await.mutation_in_progress);
    assert!(!locks.wait_idle(BRIEF).await, "a held lock is not idle");
    drop(guard);
    assert!(!status_with(&locks, &root).await.mutation_in_progress);
    assert!(locks.wait_idle(BRIEF).await);
}

/// The configured subdirectory is a second UI entry over the root's index, and
/// nothing has ever mutated through it, so its git dir was never in the
/// registry's cache. `mutationInProgress` is the UI's reconciliation oracle and
/// must still report the root's mutation.
#[tokio::test]
async fn status_for_a_sibling_root_reports_the_mutation_holding_the_shared_index() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "sub/inner.txt", b"inner\n");
    git(&root, &["add", "."]);
    git(&root, &["commit", "-qm", "sub"]);
    let sub = root.join("sub");
    let locks = SourceControlLocks::new();
    assert!(!status_with(&locks, &sub).await.mutation_in_progress);

    let guard = locks.acquire(&root).await.expect("root lock");
    assert!(
        status_with(&locks, &sub).await.mutation_in_progress,
        "the subdirectory shares the index the root is mutating"
    );
    drop(guard);
    assert!(!status_with(&locks, &sub).await.mutation_in_progress);
}

/// A linked worktree is a different index, so it must not inherit the primary
/// worktree's mutation flag.
#[tokio::test]
async fn status_for_a_linked_worktree_ignores_the_primary_worktrees_mutation() {
    let (temp, root) = init_repo_with_commit();
    let linked = temp.path().join("linked");
    git(
        &root,
        &["worktree", "add", "-q", linked.to_str().expect("utf8"), "-b", "linked"],
    );
    let locks = SourceControlLocks::new();

    let _guard = locks.acquire(&root).await.expect("root lock");
    assert!(status_with(&locks, &root).await.mutation_in_progress);
    assert!(!status_with(&locks, &linked).await.mutation_in_progress);
}

#[tokio::test]
async fn a_draining_agent_refuses_new_mutations_and_still_reads() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "base.txt", b"changed\n");
    let locks = SourceControlLocks::new();
    locks.set_draining();

    let error = try_act_with(
        &locks,
        &root,
        Action::Stage {
            scope: SourceControlScope::All,
        },
    )
    .await
    .expect_err("draining");
    assert_eq!(error.code(), "AGENT_DRAINING");
    assert_eq!(error.effect(), Some("notApplied"));
    assert!(status_with(&locks, &root).await.index.is_empty());
    assert_eq!(read(&root, "base.txt"), b"changed\n");
}

/// The residue a shutdown reports is counted per physical index, not per
/// configured root: two worktrees mutating at once are two, and nothing is
/// still held once both guards are gone.
#[tokio::test]
async fn the_busy_count_is_the_shutdown_residue() {
    let (temp, root) = init_repo_with_commit();
    let linked = temp.path().join("linked");
    git(
        &root,
        &["worktree", "add", "-q", linked.to_str().expect("utf8"), "-b", "linked"],
    );
    let locks = SourceControlLocks::new();
    assert_eq!(locks.busy_count(), 0);

    let root_guard = locks.acquire(&root).await.expect("root lock");
    assert_eq!(locks.busy_count(), 1);
    let linked_guard = locks.acquire(&linked).await.expect("linked lock");
    assert_eq!(locks.busy_count(), 2);
    assert!(!locks.wait_idle(BRIEF).await);

    drop(linked_guard);
    drop(root_guard);
    assert!(locks.wait_idle(BRIEF).await);
    assert_eq!(locks.busy_count(), 0);
}
