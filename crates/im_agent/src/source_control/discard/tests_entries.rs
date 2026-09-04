// Path: crates/im_agent/src/source_control/discard/tests_entries.rs
// Description: Delete tests: what the quarantine holds afterwards, what is refused, and what a half-applied delete reports

use super::entries::quarantine_entries;
use super::quarantine::sweep_stale_quarantine;
use crate::source_control::tests_support::*;
use crate::source_control::SourceControlLocks;

fn owned(paths: &[&str]) -> Vec<String> {
    paths.iter().map(|path| path.to_string()).collect()
}

/// Deleting is not unlinking. The file leaves the worktree, and what is left
/// behind is this operation's quarantine directory: the bytes under
/// `retained`, and a `verified` marker whose two lines say which path they
/// came from and that a delete is what took them.
#[tokio::test]
async fn a_deleted_file_is_retained_beside_a_marker_that_says_delete() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "app/a.txt", b"kept bytes\n");
    let locks = SourceControlLocks::new();

    let removed = quarantine_entries(&root, &owned(&["app/a.txt"]), &locks)
        .await
        .expect("delete");

    assert_eq!(removed, vec!["app/a.txt".to_string()]);
    assert!(!root.join("app/a.txt").exists());
    let operation = only_operation(&root.join(".git"));
    assert_eq!(text(&operation.join("retained")), "kept bytes\n");
    assert_eq!(
        text(&operation.join("verified")),
        "app/a.txt\ndelete\n",
        "the marker names the path and the plan the sweep will report"
    );
    assert!(
        !operation.join("claimed").exists(),
        "the claim is superseded by the retained bytes"
    );
}

/// A folder is claimed whole by the one rename that moves it, and the
/// directory holding it is an ordinary finished operation: once the delete
/// that owned it has ended, the next process's sweep releases it like any
/// other authorized destruction.
#[tokio::test]
async fn a_deleted_folder_is_claimed_whole_and_released_by_the_sweep() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "app/assets/deep/a.png", b"png");
    let git_dir = root.join(".git");
    let locks = SourceControlLocks::new();

    let removed = quarantine_entries(&root, &owned(&["app/assets"]), &locks)
        .await
        .expect("delete");

    assert_eq!(removed, vec!["app/assets".to_string()]);
    assert!(!root.join("app/assets").exists());
    let operation = only_operation(&git_dir);
    assert_eq!(text(&operation.join("retained/deep/a.png")), "png");

    // The next process created none of this, so its sweep finishes exactly
    // the destruction the marker authorized.
    sweep_stale_quarantine(&git_dir, &SourceControlLocks::new()).await;
    assert!(quarantine_dirs(&git_dir).is_empty());
}

/// The finding this ordering closes: a delete takes no status read of its own,
/// so the very next status read is often this process's *first*, and it is the
/// one that triggers the sweep. Nothing about that ordering may cost the user
/// the bytes they just deleted — the recovery window is until the next agent
/// start, not until the next refresh.
#[tokio::test]
async fn deleted_bytes_survive_the_status_read_that_follows_the_delete() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "app/a.txt", b"wanted back\n");
    let git_dir = root.join(".git");
    let locks = SourceControlLocks::new();

    quarantine_entries(&root, &owned(&["app/a.txt"]), &locks)
        .await
        .expect("delete");
    let operation = only_operation(&git_dir);

    sweep_stale_quarantine(&git_dir, &locks).await;
    assert_eq!(
        text(&operation.join("retained")),
        "wanted back\n",
        "this process's own sweep never releases what this process quarantined"
    );

    sweep_stale_quarantine(&git_dir, &SourceControlLocks::new()).await;
    assert!(
        quarantine_dirs(&git_dir).is_empty(),
        "the next agent start is what releases them"
    );
}

/// Every entry is validated before the first claim, so one bad path in a
/// selection refuses the whole action with nothing touched.
#[tokio::test]
async fn a_path_that_leaves_the_root_or_names_git_is_refused_before_any_claim() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "app/a.txt", b"a");
    let locks = SourceControlLocks::new();

    for paths in [
        vec!["app/a.txt", "../outside.txt"],
        vec!["app/a.txt", ".git/config"],
        vec!["app/a.txt", "app/.GIT/config"],
        vec!["app/a.txt", "app/gone.txt"],
        vec![],
    ] {
        let expected = if paths.iter().any(|path| path.ends_with("gone.txt")) {
            "ENTRY_NOT_FOUND"
        } else {
            "INVALID_PATH"
        };
        let error = quarantine_entries(&root, &owned(&paths), &locks)
            .await
            .expect_err("refusal");
        assert_eq!(error.code(), expected, "{paths:?}");
        assert_eq!(error.effect(), Some("notApplied"), "{paths:?}");
        assert_eq!(read(&root, "app/a.txt"), b"a", "{paths:?}");
        assert!(quarantine_dirs(&root.join(".git")).is_empty(), "{paths:?}");
    }
}

/// A folder and something inside it: the folder is claimed first and takes the
/// second entry with it, so the second claim finds nothing. That is no longer
/// a refusal anyone can call safe — the action is half-applied — so the effect
/// is unknown and `details.applied` names what already left.
#[tokio::test]
async fn a_failure_after_one_removal_reports_unknown_and_what_landed() {
    let (_temp, root) = init_repo_with_commit();
    write(&root, "app/assets/a.png", b"png");
    let locks = SourceControlLocks::new();

    let error = quarantine_entries(&root, &owned(&["app/assets", "app/assets/a.png"]), &locks)
        .await
        .expect_err("half applied");

    assert_eq!(error.code(), "ENTRY_NOT_FOUND");
    assert_eq!(error.effect(), Some("unknown"));
    assert_eq!(
        error.details().and_then(|details| details.get("applied")),
        Some(&serde_json::json!(["app/assets"]))
    );
    assert!(!root.join("app/assets").exists());
}
