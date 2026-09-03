// Path: crates/im_agent/src/source_control/discard/tests_quarantine.rs
// Description: Real-git tests for the discard quarantine's phase files, its per-target directories, and retention

use super::quarantine::claimed_file;
use crate::source_control::tests_support::*;
use crate::source_control::SourceControlLocks;

/// A successful discard keeps the bytes it destroyed, beside a marker saying
/// what they were matched against and what was done at the original path. That
/// pair is the whole of the safety net: the bytes to hand back, and the record
/// that removing them later finishes something the user actually authorized.
#[tokio::test]
async fn a_successful_discard_retains_the_reviewed_bytes_and_what_authorized_removing_them() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "base.txt", b"edited\n");
    act(&root, discard_now(&root, &["base.txt"])).await;
    assert_eq!(read(&root, "base.txt"), b"base\n");

    let operation = only_operation(&root.join(".git"));
    assert_eq!(text(&operation.join("verified")), "base.txt\nrestore\n");
    assert_eq!(text(&operation.join("retained")), "edited\n");
    assert!(
        !claimed_file(&operation).exists(),
        "an unverified claim must not still be the name on disk once the discard finished"
    );
}

/// An untracked file's removal is retained the same way, under the plan that
/// names what actually happened to it: nothing restored it, the claim simply
/// became the removal.
#[tokio::test]
async fn discarding_an_untracked_file_retains_it_under_a_remove_untracked_marker() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "scratch.txt", b"untracked\n");
    let outcome = act(&root, discard_now(&root, &["scratch.txt"])).await;
    assert!(!root.join("scratch.txt").exists());
    assert!(outcome.status.worktree.is_empty());

    let operation = only_operation(&root.join(".git"));
    assert_eq!(text(&operation.join("verified")), "scratch.txt\nremove-untracked\n");
    assert_eq!(text(&operation.join("retained")), "untracked\n");
}

/// Every phase file is a fixed name, so two claiming targets sharing one
/// directory would have the second target's retention replace the first
/// target's bytes and overwrite the marker that says what they were. Each
/// target of one action therefore owns its own directory, and both survive
/// the action intact.
#[tokio::test]
async fn each_claiming_target_of_one_action_gets_its_own_quarantine_directory() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "second.txt", b"second\n");
    git(&root, &["add", "."]);
    git(&root, &["commit", "-qm", "second"]);
    write(&root, "base.txt", b"edited base\n");
    write(&root, "second.txt", b"edited second\n");

    act(&root, discard_now(&root, &["base.txt", "second.txt"])).await;

    let mut kept: Vec<(String, String)> = quarantine_entries(&root.join(".git"))
        .iter()
        .map(|operation| {
            (
                text(&operation.join("verified")),
                text(&operation.join("retained")),
            )
        })
        .collect();
    kept.sort();
    assert_eq!(
        kept,
        vec![
            ("base.txt\nrestore\n".to_string(), "edited base\n".to_string()),
            (
                "second.txt\nrestore\n".to_string(),
                "edited second\n".to_string()
            ),
        ]
    );
}

/// Retention lasts until the next agent start, not forever and not only until
/// the next refresh: the directory a discard leaves behind survives later
/// status reads by the process that made it, and is swept by the first status
/// read of the next one.
#[tokio::test]
async fn retained_bytes_survive_this_process_and_are_swept_by_the_next() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "base.txt", b"edited\n");
    let locks = SourceControlLocks::new();
    try_act_with(&locks, &root, discard_now(&root, &["base.txt"]))
        .await
        .expect("discard");
    let operation = only_operation(&root.join(".git"));

    status_with(&locks, &root).await;
    assert!(
        operation.exists(),
        "the bytes a user may still want back outlive the refresh that follows the discard"
    );

    status_with(&SourceControlLocks::new(), &root).await;
    assert!(!operation.exists(), "the next process's first status releases them");
}

/// The step the `verified` marker announced could not run: the claim already
/// emptied the worktree path, so the quarantined bytes are the only copy of
/// what the user reviewed — and the marker beside them would tell the next
/// start's sweep it may destroy them. The failure must move them out of that
/// reach and say where they went, or the user's file is gone at the next
/// start with nothing to show for it.
///
/// The restore is stopped by a required smudge filter that always fails, which
/// is the closest real repository configuration to `git restore` dying after
/// the discard was authorized.
#[cfg(unix)]
#[tokio::test]
async fn a_restore_that_cannot_run_holds_the_reviewed_bytes_out_of_the_sweep() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, ".gitattributes", b"*.txt filter=fail\n");
    git(&root, &["add", ".gitattributes"]);
    git(&root, &["commit", "-qm", "attributes"]);
    git(&root, &["config", "filter.fail.smudge", "false"]);
    git(&root, &["config", "filter.fail.required", "true"]);
    write(&root, "base.txt", b"edited\n");
    assert!(
        !git_succeeds(&root, &["restore", "--worktree", "--", "base.txt"]),
        "the premise: this repository's `git restore --worktree` really cannot run"
    );
    write(&root, "base.txt", b"edited\n");

    let error = try_act(&root, discard_now(&root, &["base.txt"]))
        .await
        .expect_err("the restore cannot run");

    assert_eq!(error.effect(), Some("unknown"));
    let operation = only_operation(&root.join(".git"));
    let held = operation.join("unrestored");
    assert!(
        error.message().contains(&held.display().to_string()),
        "the failure must name where the reviewed bytes are: {}",
        error.message()
    );
    assert_eq!(std::fs::read(&held).expect("held file"), b"edited\n");
    assert!(
        !claimed_file(&operation).exists(),
        "the bytes must stop looking like a destruction the sweep may finish"
    );

    status_with(&SourceControlLocks::new(), &root).await;
    assert_eq!(
        std::fs::read(&held).expect("held file"),
        b"edited\n",
        "the next process's first status read keeps the only copy the user has"
    );
}
