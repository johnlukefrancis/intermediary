// Path: crates/im_agent/src/repos/git_ignore/git_ignore_tests.rs
// Description: Real-repository tests for the Git ignore filter: info/exclude, tracked override, and event reload

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use notify::event::{AccessKind, AccessMode, CreateKind, EventKind};
use notify::Event;

use super::GitIgnoreFilter;
use crate::logging::{LogConfig, LogLevel, Logger};
use crate::repos::source_control_watch::{load_tracked_paths, TrackedPathSet};

fn git(cwd: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .status()
        .expect("run git");
    assert!(status.success(), "git command failed: {args:?}");
}

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("create dirs");
    std::fs::write(path, contents).expect("write file");
}

async fn filter_for(root: &Path, logs: &Path) -> GitIgnoreFilter {
    let logger = Logger::init(LogConfig {
        log_dir: logs.to_path_buf(),
        file_stem: "agent_test",
        min_level: LogLevel::Warn,
        emit_stdio: false,
    })
    .await
    .expect("logger");
    let tracked = TrackedPathSet::empty();
    tracked.store(load_tracked_paths(root).await.expect("tracked paths"));
    let filter = GitIgnoreFilter::new(root.to_path_buf(), tracked, logger);
    filter.load_now().await;
    filter
}

#[tokio::test]
async fn suppresses_ignored_untracked_paths_from_every_git_source() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join("repo");
    std::fs::create_dir_all(&root).expect("create repo");
    git(&root, &["init", "-q"]);
    write(&root, ".gitignore", "/Saved/\n");
    write(&root, "Source/.gitignore", "*.gen.h\n");
    write(&root, ".git/info/exclude", "*.scratch\n");
    write(&root, "Saved/Config/keep.ini", "k");
    git(&root, &["add", ".gitignore", "Source/.gitignore"]);
    git(&root, &["add", "-f", "Saved/Config/keep.ini"]);

    let filter = filter_for(&root, &temp.path().join("logs")).await;
    assert!(filter.is_suppressed("Saved/Logs/Game.log"));
    assert!(filter.is_suppressed("Source/Game/Thing.gen.h"));
    assert!(filter.is_suppressed("notes.scratch"), "info/exclude");
    assert!(
        !filter.is_suppressed("Saved/Config/keep.ini"),
        "tracked wins"
    );
    assert!(!filter.is_suppressed("Source/Game/Thing.h"));
}

#[tokio::test]
async fn subfolder_root_uses_top_level_coordinates() {
    let temp = tempfile::tempdir().expect("tempdir");
    let top = temp.path().join("repo");
    std::fs::create_dir_all(&top).expect("create repo");
    git(&top, &["init", "-q"]);
    write(&top, ".gitignore", "/Game/Saved/\n/Top.log\n");
    write(&top, "Game/.gitignore", "/Binaries/\n*.tmp\n");
    write(&top, "Game/Saved/Config/keep.ini", "k");
    git(&top, &["add", ".gitignore", "Game/.gitignore"]);
    git(&top, &["add", "-f", "Game/Saved/Config/keep.ini"]);

    let root = top.join("Game/Plugin");
    std::fs::create_dir_all(&root).expect("create plugin dir");
    let filter = filter_for(&top.join("Game"), &temp.path().join("logs")).await;
    assert!(
        filter.is_suppressed("Saved/Logs/Game.log"),
        "anchored at top"
    );
    assert!(
        filter.is_suppressed("Binaries/Win64/a.dll"),
        "root's own .gitignore"
    );
    assert!(
        !filter.is_suppressed("Saved/Config/keep.ini"),
        "tracked wins"
    );
    assert!(
        !filter.is_suppressed("Top.log"),
        "anchored to the top level only"
    );

    let nested = filter_for(&root, &temp.path().join("logs2")).await;
    assert!(
        !nested.is_suppressed("Saved/x.log"),
        "Game/Plugin/Saved is not ignored"
    );
    assert!(
        !nested.is_suppressed("Binaries/a.dll"),
        "Game/.gitignore anchors to Game"
    );
    assert!(
        nested.is_suppressed("Source/a.tmp"),
        "ancestor .gitignore applies"
    );
}

#[tokio::test]
async fn untracked_gitignore_event_reloads_the_rules() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join("repo");
    std::fs::create_dir_all(&root).expect("create repo");
    git(&root, &["init", "-q"]);

    let filter = filter_for(&root, &temp.path().join("logs")).await;
    assert!(!filter.is_suppressed("Plugin/Intermediate/a.obj"));

    write(&root, "Plugin/.gitignore", "Intermediate/\n");
    let event =
        Event::new(EventKind::Create(CreateKind::File)).add_path(root.join("Plugin/.gitignore"));
    filter.note_event(&event);
    for _ in 0..200 {
        if filter.is_suppressed("Plugin/Intermediate/a.obj") {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("reload never applied the new .gitignore");
}

#[tokio::test]
async fn an_open_of_a_gitignore_never_schedules_a_reload() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join("repo");
    std::fs::create_dir_all(&root).expect("create repo");
    git(&root, &["init", "-q"]);

    let filter = filter_for(&root, &temp.path().join("logs")).await;
    write(&root, "Plugin/.gitignore", "Intermediate/\n");
    let event = Event::new(EventKind::Access(AccessKind::Open(AccessMode::Any)))
        .add_path(root.join("Plugin/.gitignore"));
    filter.note_event(&event);
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(!filter.is_suppressed("Plugin/Intermediate/a.obj"), "an open must not reload");
}
