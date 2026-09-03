// Path: crates/im_agent/src/repos/repo_watcher_tests.rs
// Description: Unit tests for the repo watcher's initial-entries ignore filtering

use super::repo_watcher::filter_initial_entries;
use crate::protocol::{FileChangeType, FileEntry, FileKind};
use crate::repos::ignore_matcher::IgnoreMatcher;

#[test]
fn test_filter_initial_entries_applies_ignore_globs() {
    let matcher = IgnoreMatcher::new(&["**/*.cpp".to_string()]).expect("valid ignore glob");
    let entries = vec![
        FileEntry {
            path: "src\\engine\\render.cpp".to_string(),
            kind: FileKind::Code,
            change_type: FileChangeType::Add,
            mtime: "2026-02-06T00:00:00Z".to_string(),
            size_bytes: Some(8),
            activity: None,
        },
        FileEntry {
            path: "docs/readme.md".to_string(),
            kind: FileKind::Docs,
            change_type: FileChangeType::Add,
            mtime: "2026-02-06T00:00:00Z".to_string(),
            size_bytes: Some(12),
            activity: None,
        },
    ];

    let filtered = filter_initial_entries(entries, &matcher);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].path, "docs/readme.md");
}
