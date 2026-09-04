// Path: crates/im_agent/src/source_control/discard/tests_sweep.rs
// Description: Tests for the once-per-process discard quarantine sweep: what it finishes, what it spares, and what it survives

use super::claim::Claim;
use super::quarantine::{claimed_file, quarantine_root, sweep_stale_quarantine};
use crate::source_control::tests_support::*;
use crate::source_control::SourceControlLocks;

/// A quarantine directory whose marker says its content was matched against
/// the review is removed on the first status read of a later process — that is
/// finishing exactly the destruction the user authorized — and only that first
/// time: a directory that appears afterwards is left for the next process.
#[tokio::test]
async fn a_verified_quarantine_directory_is_swept_once_on_the_first_status_read() {
    let (_temp, root) = init_repo_with_commit();
    let git_dir = root.join(".git");
    let stale = quarantine_root(&git_dir, "stale-op-id", 0);
    std::fs::create_dir_all(&stale).expect("stale op dir");
    std::fs::write(stale.join("verified"), "base.txt\nrestore\n").expect("marker");
    std::fs::write(stale.join("retained"), b"leftover").expect("retained file");

    let locks = SourceControlLocks::new();
    status_with(&locks, &root).await;
    assert!(
        quarantine_dirs(&git_dir).is_empty(),
        "the finished directory is swept on the first status read"
    );

    let later = quarantine_root(&git_dir, "later-op-id", 0);
    std::fs::create_dir_all(&later).expect("later op dir");
    std::fs::write(later.join("verified"), "base.txt\nrestore\n").expect("marker");
    status_with(&locks, &root).await;
    assert_eq!(
        quarantine_dirs(&git_dir).len(),
        1,
        "the sweep runs at most once per git dir per process"
    );
}

/// A process that died between claiming a file and matching it against the
/// review left bytes nothing ever authorized destroying. The sweep must not
/// finish that job for it: with no marker, the directory stands.
#[tokio::test]
async fn the_sweep_keeps_a_claim_that_died_before_anything_verified_it() {
    let (_temp, root) = init_repo_with_commit();
    let crashed = quarantine_root(&root.join(".git"), "crashed-op-id", 0);
    std::fs::create_dir_all(&crashed).expect("crashed op dir");
    std::fs::write(claimed_file(&crashed), b"never checked\n").expect("claimed file");

    status_with(&SourceControlLocks::new(), &root).await;

    assert_eq!(
        std::fs::read(claimed_file(&crashed)).expect("claimed file"),
        b"never checked\n",
        "an unverified claim is not a destruction anyone authorized"
    );
}

/// A quarantine directory holding content a rollback could not restore is the
/// other thing the sweep must not delete: those bytes were never authorized
/// for destruction, and they are the only copy of what the user reviewed.
#[tokio::test]
async fn the_sweep_leaves_a_directory_holding_unrestored_content() {
    let (_temp, root) = init_repo_with_commit();
    let stranded = quarantine_root(&root.join(".git"), "stranded-op-id", 0);
    std::fs::create_dir_all(&stranded).expect("stranded op dir");
    std::fs::write(stranded.join("unrestored"), b"reviewed\n").expect("unrestored file");
    // The same directory also carries the marker that would otherwise
    // authorize its removal: bytes that could not be put back outrank it.
    std::fs::write(stranded.join("verified"), "base.txt\nrestore\n").expect("marker");

    status_with(&SourceControlLocks::new(), &root).await;

    assert_eq!(
        std::fs::read(stranded.join("unrestored")).expect("held file"),
        b"reviewed\n"
    );
}

/// The put-back has the same stake as a rollback: the claimed file is the only
/// copy of what the user reviewed, and that target was never authorized for
/// destruction at all. A rename that cannot land must leave those bytes
/// standing — including past the sweep that finishes authorized discards.
#[tokio::test]
async fn a_put_back_that_cannot_land_holds_the_reviewed_bytes_past_the_sweep() {
    let temp = tempfile::tempdir().expect("tempdir");
    let git_dir = temp.path().join("git");
    let quarantine = quarantine_root(&git_dir, "op-id", 0);
    std::fs::create_dir_all(&quarantine).expect("op dir");
    let quarantined = claimed_file(&quarantine);
    std::fs::write(&quarantined, b"reviewed\n").expect("claimed file");
    // The worktree the file came from is gone, so the rename cannot land.
    let repo_root = temp.path().join("repo");

    let error = Claim {
        root: quarantine.clone(),
    }
    .restore(&repo_root, "victim.txt")
    .expect_err("the put-back cannot land");

    assert_eq!(error.effect(), Some("unknown"));
    let held = quarantine.join("unrestored");
    assert!(
        error.message().contains(&held.display().to_string()),
        "the message must name where the bytes are: {}",
        error.message()
    );
    assert!(!quarantined.exists());

    sweep_stale_quarantine(&git_dir, &SourceControlLocks::new()).await;
    assert_eq!(
        std::fs::read(&held).expect("held file"),
        b"reviewed\n",
        "the sweep spares content no rollback or put-back could restore"
    );
}

/// A directory this process created is never removed by this process's sweep,
/// whether its discard is still claiming, has just returned, or finished long
/// ago: retention lasts until the *next* agent start, so no ordering of this
/// process's reads and mutations can release those bytes early. The next
/// process, which created nothing, finishes exactly the destruction the
/// `verified` marker authorized.
#[tokio::test]
async fn the_sweep_leaves_a_directory_this_process_created() {
    let temp = tempfile::tempdir().expect("tempdir");
    let git_dir = temp.path().join("git");
    let locks = SourceControlLocks::new();
    let operation = quarantine_root(&git_dir, "live-op-id", 0);
    std::fs::create_dir_all(&operation).expect("op dir");
    std::fs::write(operation.join("verified"), "base.txt\nrestore\n").expect("marker");
    std::fs::write(operation.join("retained"), b"in flight\n").expect("retained file");

    locks.register_discard_op("live-op-id");
    sweep_stale_quarantine(&git_dir, &locks).await;
    assert!(
        operation.exists(),
        "a directory this process created is never swept by this process"
    );

    sweep_stale_quarantine(&git_dir, &SourceControlLocks::new()).await;
    assert!(
        !operation.exists(),
        "the next process is the one that finishes it"
    );
}

/// The sweep runs once per git dir per process, so one directory it cannot
/// remove must not cost every other finished discard its release: those bytes
/// would then sit on disk until the next start, and the start after that would
/// hit the same wall.
#[cfg(unix)]
#[tokio::test]
async fn a_directory_the_sweep_cannot_remove_does_not_stop_the_rest() {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempfile::tempdir().expect("tempdir");
    let git_dir = temp.path().join("git");
    let stuck = quarantine_root(&git_dir, "stuck-op-id", 0);
    let sealed = stuck.join("sealed");
    std::fs::create_dir_all(&sealed).expect("sealed dir");
    std::fs::write(sealed.join("inside"), b"unreachable\n").expect("inner file");
    std::fs::write(stuck.join("verified"), "base.txt\nrestore\n").expect("marker");
    std::fs::set_permissions(&sealed, std::fs::Permissions::from_mode(0o555)).expect("seal");

    let finished = quarantine_root(&git_dir, "finished-op-id", 0);
    std::fs::create_dir_all(&finished).expect("op dir");
    std::fs::write(finished.join("verified"), "other.txt\nrestore\n").expect("marker");
    std::fs::write(finished.join("retained"), b"leftover\n").expect("retained file");

    sweep_stale_quarantine(&git_dir, &SourceControlLocks::new()).await;

    let stuck_stands = stuck.exists();
    // Restored before the assertions so the tempdir can always clean itself up.
    std::fs::set_permissions(&sealed, std::fs::Permissions::from_mode(0o755)).expect("unseal");
    assert!(stuck_stands, "the directory that could not be removed stands");
    assert!(
        !finished.exists(),
        "its neighbour is still released by the same sweep"
    );
}
