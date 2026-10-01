// Path: crates/im_agent/src/repos/import/tests_git_control.rs
// Description: Real linked-worktree imports refuse Git pointer files before every write policy

use super::tests_support::{import, worktree};
use crate::protocol::ImportConflictPolicy;
use std::fs;
use std::path::Path;
use std::process::Command;

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .expect("git");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
async fn linked_worktree_pointer_files_refuse_flat_nested_and_replace_imports() {
    let source = tempfile::tempdir().expect("source");
    let main = source.path().join("main");
    let dropped = source.path().join("dropped");
    let linked = dropped.join("linked");
    fs::create_dir_all(&main).expect("main");
    fs::create_dir_all(&dropped).expect("drop");
    git(&main, &["init", "-q"]);
    fs::write(main.join("tracked.txt"), b"original").expect("seed");
    git(&main, &["add", "tracked.txt"]);
    git(
        &main,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-qm",
            "seed",
        ],
    );
    git(
        &main,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "linked",
            linked.to_str().expect("path"),
        ],
    );
    assert!(linked.join(".git").is_file());
    let pointer = fs::read(linked.join(".git")).expect("pointer");
    let index = fs::read(main.join(".git/worktrees/linked/index")).expect("original index");

    for (input, dest) in [(&linked, "linked"), (&dropped, "dropped/linked")] {
        for replace in [false, true] {
            let repo = worktree();
            let destination = repo.path().join("app").join(dest);
            if replace {
                fs::create_dir_all(&destination).expect("existing destination");
                fs::write(destination.join(".git"), b"existing pointer")
                    .expect("existing Git file");
                fs::write(destination.join("tracked.txt"), b"existing content")
                    .expect("existing content");
            }
            let policy = if replace {
                ImportConflictPolicy::Replace(vec![format!("app/{dest}/tracked.txt")])
            } else {
                ImportConflictPolicy::Refuse
            };
            let error = import(
                repo.path(),
                "app",
                &[input.to_string_lossy().into_owned()],
                policy,
            )
            .await
            .expect_err("Git pointer import");
            assert_eq!(error.code(), "INVALID_PATH");
            assert_eq!(error.effect(), Some("notApplied"));
            if replace {
                assert_eq!(
                    fs::read(destination.join(".git")).expect("preserved Git file"),
                    b"existing pointer"
                );
                assert_eq!(
                    fs::read(destination.join("tracked.txt")).expect("preserved content"),
                    b"existing content"
                );
            } else {
                assert_eq!(
                    fs::read_dir(repo.path().join("app"))
                        .expect("destination entries")
                        .count(),
                    0
                );
            }
        }
    }
    assert_eq!(
        fs::read(linked.join(".git")).expect("unchanged source pointer"),
        pointer
    );
    assert_eq!(
        fs::read(main.join(".git/worktrees/linked/index")).expect("unchanged index"),
        index
    );
}
