// Path: crates/im_agent/src/repos/repo_watcher_startup_tests.rs
// Description: Unit test that repo watch registration never follows symlinks out of the repo root

use super::create_watcher;
use notify::{RecursiveMode, Watcher};
use std::time::Duration;

/// A link out of the root (`work -> /mnt/j/...`) cost 60 s of recursive inotify registration.
#[cfg(unix)]
#[tokio::test]
async fn recursive_watch_does_not_follow_symlinked_dirs() {
    let outside = tempfile::tempdir().expect("outside dir");
    let repo = tempfile::tempdir().expect("repo dir");
    std::os::unix::fs::symlink(outside.path(), repo.path().join("work")).expect("symlink");

    let (mut watcher, mut events) = create_watcher().expect("watcher");
    watcher
        .watch(repo.path(), RecursiveMode::Recursive)
        .expect("watch repo");

    std::fs::write(outside.path().join("linked.txt"), "x").expect("write outside");
    std::fs::write(repo.path().join("inside.txt"), "x").expect("write inside");

    let mut saw_inside = false;
    while let Ok(Some(message)) =
        tokio::time::timeout(Duration::from_millis(500), events.recv()).await
    {
        let event = message.expect("watch event");
        for path in &event.paths {
            assert!(!path.ends_with("linked.txt"), "followed symlink: {path:?}");
            saw_inside |= path.ends_with("inside.txt");
        }
    }
    assert!(saw_inside, "repo-local write must still be observed");
}
