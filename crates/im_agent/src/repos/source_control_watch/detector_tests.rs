// Path: crates/im_agent/src/repos/source_control_watch/detector_tests.rs
// Description: Unit tests for the source-control change detector (tracked-set override, git metadata allowlist)

use super::detector::SourceControlChangeDetector;
use super::TrackedPathSet;
use notify::event::{AccessKind, CreateKind, ModifyKind, RenameMode};
use notify::{Event, EventKind};
use std::path::{Path, PathBuf};

const ROOT: &str = "/repo";
const EXTERNAL_GIT_DIR: &str = "/main/.git/worktrees/feature";

fn detector(ignore_globs: &[&str]) -> SourceControlChangeDetector {
    detector_with_tracked(ignore_globs, &[])
}

fn detector_with_tracked(
    ignore_globs: &[&str],
    tracked_paths: &[&str],
) -> SourceControlChangeDetector {
    let globs: Vec<String> = ignore_globs.iter().map(|glob| glob.to_string()).collect();
    let tracked = TrackedPathSet::empty();
    tracked.store(tracked_paths.iter().map(|path| path.to_string()).collect());
    SourceControlChangeDetector::new(
        Path::new(ROOT),
        vec![PathBuf::from(EXTERNAL_GIT_DIR)],
        &globs,
        tracked,
    )
    .expect("detector builds")
}

fn modified(path: &str) -> Event {
    Event::new(EventKind::Modify(ModifyKind::Any)).add_path(PathBuf::from(path))
}

#[test]
fn working_tree_changes_fire() {
    let detector = detector(&[]);
    for path in [
        "/repo/logs/app.log",
        "/repo/dist/index.js",
        "/repo/Cargo.lock",
        "/repo/build/out.bin",
        "/repo/src/main.rs",
    ] {
        assert!(detector.affects(&modified(path)), "expected fire: {path}");
    }
}

#[test]
fn git_metadata_changes_fire() {
    let detector = detector(&[]);
    for path in [
        "/repo/.git/index",
        "/repo/.git/index.lock",
        "/repo/.git/HEAD",
        "/repo/.git/MERGE_HEAD",
        "/repo/.git/packed-refs",
        "/repo/.git/refs/heads/main",
        "/repo/.git/refs/heads/main.lock",
        "/repo/.git/config",
        "/repo/.git/info/exclude",
        "/repo/.git/worktrees/feature/config",
        "/repo/.git",
    ] {
        assert!(detector.affects(&modified(path)), "expected fire: {path}");
    }
}

#[test]
fn git_dir_creation_fires() {
    let detector = detector(&[]);
    let event = Event::new(EventKind::Create(CreateKind::Folder)).add_path("/repo/.git".into());
    assert!(detector.affects(&event));
}

#[test]
fn structural_and_git_noise_do_not_fire() {
    let detector = detector(&[]);
    for path in [
        "/repo/target/x.o",
        "/repo/node_modules/a/b.js",
        "/repo/.git/objects/ab/cd",
        "/repo/.git/logs/HEAD",
        "/repo/.git/hooks/pre-commit",
        "/repo/.git/worktrees/feature/gitdir",
        "/elsewhere/file.rs",
    ] {
        assert!(!detector.affects(&modified(path)), "expected quiet: {path}");
    }
}

#[test]
fn access_events_do_not_fire() {
    let detector = detector(&[]);
    let event =
        Event::new(EventKind::Access(AccessKind::Any)).add_path("/repo/src/main.rs".into());
    assert!(!detector.affects(&event));
}

#[test]
fn rename_events_fire() {
    let detector = detector(&[]);
    let event = Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Both)))
        .add_path("/repo/.git/index.lock".into())
        .add_path("/repo/.git/index".into());
    assert!(detector.affects(&event));
}

#[test]
fn external_git_dir_applies_metadata_rule_only() {
    let detector = detector(&[]);
    assert!(detector.affects(&modified("/main/.git/worktrees/feature/HEAD")));
    assert!(detector.affects(&modified("/main/.git/worktrees/feature/refs/bisect/bad")));
    assert!(!detector.affects(&modified("/main/.git/worktrees/feature/objects/x")));
    assert!(!detector.affects(&modified("/main/.git/worktrees/feature/logs/HEAD")));
}

#[test]
fn configured_ignore_globs_suppress() {
    let detector = detector(&["**/generated/**"]);
    assert!(!detector.affects(&modified("/repo/src/generated/api.rs")));
    assert!(detector.affects(&modified("/repo/src/api.rs")));
}

#[test]
fn tracked_file_under_a_structural_ignore_still_fires() {
    let detector = detector_with_tracked(&[], &["target/keep.rs"]);
    assert!(
        detector.affects(&modified("/repo/target/keep.rs")),
        "a tracked path must emit even under the target/ default"
    );
}

#[test]
fn untracked_file_under_a_structural_ignore_stays_quiet() {
    let detector = detector_with_tracked(&[], &["target/keep.rs"]);
    assert!(
        !detector.affects(&modified("/repo/target/other.rs")),
        "an untracked path under target/ is still noise"
    );
}

#[test]
fn index_change_is_reported_as_an_index_change_and_nothing_else_is() {
    let detector = detector(&[]);
    assert!(detector.is_index_change(&modified("/repo/.git/index")));
    assert!(detector.is_index_change(&modified("/repo/.git/index.lock")));
    assert!(detector.is_index_change(&modified(
        "/main/.git/worktrees/feature/index"
    )));
    assert!(!detector.is_index_change(&modified("/repo/.git/HEAD")));
    assert!(!detector.is_index_change(&modified("/repo/.git/config")));
    assert!(!detector.is_index_change(&modified("/repo/src/main.rs")));
}
