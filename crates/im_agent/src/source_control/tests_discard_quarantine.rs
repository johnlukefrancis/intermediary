// Path: crates/im_agent/src/source_control/tests_discard_quarantine.rs
// Description: Real-git tests for the discard quarantine directory's cleanup and startup sweep

use super::tests_support::*;
use super::SourceControlLocks;

fn quarantine_dir(root: &std::path::Path) -> std::path::PathBuf {
    root.join(".git").join("intermediary-discard")
}

fn entries(dir: &std::path::Path) -> Vec<std::fs::DirEntry> {
    match std::fs::read_dir(dir) {
        Ok(read) => read.collect::<Result<Vec<_>, _>>().expect("read dir"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => panic!("read {}: {error}", dir.display()),
    }
}

/// A successful discard's own operation directory is removed once every
/// target has been released: nothing is left for a later sweep to find.
#[tokio::test]
async fn quarantine_directory_is_removed_after_a_successful_discard() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "base.txt", b"edited\n");
    act(&root, discard_now(&root, &["base.txt"])).await;
    assert!(
        entries(&quarantine_dir(&root)).is_empty(),
        "no operation directories survive a clean discard"
    );
}

/// An untracked file discard goes through the same claim/quarantine/release
/// path as a tracked restore: the file is removed, and no quarantine entry
/// survives the successful removal.
#[tokio::test]
async fn discarding_an_untracked_file_leaves_no_quarantine_residue() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "scratch.txt", b"untracked\n");
    let outcome = act(&root, discard_now(&root, &["scratch.txt"])).await;
    assert!(!root.join("scratch.txt").exists());
    assert!(outcome.status.worktree.is_empty());
    assert!(
        entries(&quarantine_dir(&root)).is_empty(),
        "no operation directories survive a clean discard"
    );
}

/// An `<opId>` directory left behind by an earlier process (a crash mid-claim)
/// is removed the first time this process reads this repository's status, and
/// only that first time — a directory that appears afterwards is left for the
/// next process to find.
#[tokio::test]
async fn stale_quarantine_directories_are_swept_once_on_the_first_status_read() {
    let (_temp, root) = init_repo_with_commit();
    let quarantine = quarantine_dir(&root);
    let stale = quarantine.join("stale-op-id");
    std::fs::create_dir_all(&stale).expect("stale op dir");
    std::fs::write(stale.join("claimed"), b"leftover").expect("leftover file");
    assert_eq!(entries(&quarantine).len(), 1);

    let locks = SourceControlLocks::new();
    status_with(&locks, &root).await;
    assert!(
        entries(&quarantine).is_empty(),
        "the stale directory is swept on the first status read"
    );

    // A directory that appears after the first read is not this process's
    // concern again until it restarts.
    let later = quarantine.join("later-op-id");
    std::fs::create_dir_all(&later).expect("later op dir");
    status_with(&locks, &root).await;
    assert_eq!(
        entries(&quarantine).len(),
        1,
        "the sweep runs at most once per git dir per process"
    );
}

/// A quarantine directory holding content a rollback could not restore is the
/// one thing the sweep must not delete: those bytes were never authorized for
/// destruction, and they are the only copy of what the user reviewed.
#[tokio::test]
async fn the_sweep_leaves_a_directory_holding_unrestored_content() {
    let (_temp, root) = init_repo_with_commit();
    let quarantine = quarantine_dir(&root);
    let stranded = quarantine.join("stranded-op-id");
    std::fs::create_dir_all(&stranded).expect("stranded op dir");
    std::fs::write(stranded.join("unrestored"), b"reviewed\n").expect("unrestored file");
    let authorized = quarantine.join("crashed-op-id");
    std::fs::create_dir_all(&authorized).expect("crashed op dir");
    std::fs::write(authorized.join("claimed"), b"verified\n").expect("claimed file");

    status_with(&SourceControlLocks::new(), &root).await;

    assert!(
        !authorized.exists(),
        "a verified claim is still swept: removing it finishes a destruction the user authorized"
    );
    assert_eq!(
        std::fs::read(stranded.join("unrestored")).expect("held file"),
        b"reviewed\n"
    );
}
