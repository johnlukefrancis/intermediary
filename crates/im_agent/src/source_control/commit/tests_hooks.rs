// Path: crates/im_agent/src/source_control/commit/tests_hooks.rs
// Description: Real-git tests for what a commit hook did to a landed commit: reviewed rewrites and unreviewed additions are both reported, never undone

#![cfg(unix)]

use crate::source_control::tests_support::*;

/// A repository whose configured root is the `sub/` subdirectory, with one
/// in-root and one outside-root path staged: the shape in which the UI shows
/// its outside-root warning and the user acknowledges it before committing.
fn repo_with_an_acknowledged_outside_root_path() -> (tempfile::TempDir, std::path::PathBuf) {
    let (temp, root) = init_repo_with_commit();
    write(&root, "sub/reviewed.txt", b"reviewed\n");
    write(&root, "outside.txt", b"outside\n");
    git(&root, &["add", "."]);
    git(&root, &["commit", "-qm", "add sub and outside"]);

    write(&root, "sub/reviewed.txt", b"reviewed again\n");
    write(&root, "outside.txt", b"outside again\n");
    git(&root, &["add", "."]);
    (temp, root)
}

/// A `pre-commit` hook that reformats an already-reviewed file and re-stages
/// it changes content the user had in front of them: the commit lands and the
/// rewrite is reported through `hookChangedPaths`.
#[tokio::test]
async fn a_hook_that_rewrites_a_reviewed_path_is_applied_with_hook_changed_paths() {
    let (_temp, root) = init_repo_with_commit();
    write_hook(
        &root,
        "pre-commit",
        "printf 'reformatted\\n' >> reviewed.txt\ngit add -- reviewed.txt\n",
    );
    write(&root, "reviewed.txt", b"before\n");
    git(&root, &["add", "reviewed.txt"]);

    let action = commit_now(&root, "Add reviewed").await;
    let outcome = act(&root, action).await;

    assert_eq!(
        outcome.hook_changed_paths,
        Some(vec!["reviewed.txt".to_string()])
    );
    assert_eq!(outcome.hook_added_paths, None);
    let head = git_stdout(&root, &["rev-parse", "HEAD"]);
    assert_eq!(outcome.commit_sha.as_deref(), Some(head.as_str()));
    assert_eq!(read(&root, "reviewed.txt"), b"before\nreformatted\n");
    assert_eq!(git_stdout(&root, &["log", "-1", "--format=%s"]), "Add reviewed");
}

/// A `pre-commit` hook that stages a path the user never reviewed puts that
/// path in the commit. The commit is history the moment Git writes it, so it
/// stands — and the path the user never saw is named in `hookAddedPaths`
/// rather than being silently swallowed or rewound behind their back.
#[tokio::test]
async fn a_hook_that_adds_an_unreviewed_path_is_applied_with_hook_added_paths() {
    let (_temp, root) = init_repo_with_commit();
    write_hook(
        &root,
        "pre-commit",
        "printf 'unreviewed\\n' > generated.txt\ngit add -- generated.txt\n",
    );
    write(&root, "reviewed.txt", b"reviewed\n");
    git(&root, &["add", "reviewed.txt"]);
    let before_head = git_stdout(&root, &["rev-parse", "HEAD"]);

    let action = commit_now(&root, "Add reviewed").await;
    let outcome = act(&root, action).await;

    assert_eq!(
        outcome.hook_added_paths,
        Some(vec!["generated.txt".to_string()])
    );
    assert_eq!(outcome.hook_changed_paths, None);
    let head = git_stdout(&root, &["rev-parse", "HEAD"]);
    assert_ne!(head, before_head, "the commit stands");
    assert_eq!(outcome.commit_sha.as_deref(), Some(head.as_str()));
    assert_eq!(
        git_stdout(&root, &["show", "--name-only", "--format=", "HEAD"]),
        "generated.txt\nreviewed.txt",
        "the hook's path rode along inside the commit"
    );
}

/// A subdirectory root reports both lists in repository-root path space, the
/// space `diff-tree` names, so the UI is never handed a path relative to a
/// root the hook knew nothing about. Here one hook does both things at once:
/// it rewrites the outside-root path the user acknowledged, and adds a second
/// one nobody reviewed.
#[tokio::test]
async fn a_subdirectory_root_reports_rewrites_and_additions_from_the_repository_root() {
    let (_temp, root) = repo_with_an_acknowledged_outside_root_path();
    write_hook(
        &root,
        "pre-commit",
        "printf 'reformatted\\n' >> outside.txt\nprintf 'sneaky\\n' > sneaky.txt\ngit add -- outside.txt sneaky.txt\n",
    );

    let sub = root.join("sub");
    let reviewed = status(&sub).await;
    assert_eq!(
        reviewed.omitted.staged_outside_root, 1,
        "outside.txt is staged, and the UI asked before sending"
    );

    let outcome = act(&sub, commit_for(&reviewed, "Update reviewed")).await;

    assert_eq!(
        outcome.hook_changed_paths,
        Some(vec!["outside.txt".to_string()]),
        "an already-reviewed path, named from the repository root"
    );
    assert_eq!(
        outcome.hook_added_paths,
        Some(vec!["sneaky.txt".to_string()])
    );
    let head = git_stdout(&root, &["rev-parse", "HEAD"]);
    assert_eq!(outcome.commit_sha.as_deref(), Some(head.as_str()));
    assert_eq!(read(&root, "outside.txt"), b"outside again\nreformatted\n");
    assert_eq!(
        git_stdout(&root, &["log", "-1", "--format=%s"]),
        "Update reviewed"
    );
}
