// Path: crates/im_agent/src/repos/delta/delta_patch_tests.rs
// Description: Patch grammar, truncation and settled-read tests for the delta pipeline

use std::fs;
use std::time::{Duration, Instant};

use tempfile::tempdir;

use crate::protocol::OpaqueReason;

use super::delta_read::{read_settled, stamp_moved, FileStamp, ReadOutcome};
use super::unified_patch::{all_added_patch, all_removed_patch, compute_patch};
use super::{DIFF_DEADLINE, PATCH_MAX_BYTES};

#[test]
fn patch_grammar_and_truncation() {
    let old = (0..40)
        .map(|line| format!("line {line}\n"))
        .collect::<String>();
    let new = old.replace("line 20\n", "line twenty\n");
    let patched = compute_patch(&old, &new, Instant::now() + DIFF_DEADLINE);

    assert!(!patched.truncated);
    assert_eq!(patched.stats.added, 1);
    assert_eq!(patched.stats.removed, 1);
    assert_eq!(patched.stats.hunks, 1);
    assert_eq!(patched.stats.new_lines, 40);
    let first = patched.patch.lines().next().expect("a hunk header first");
    assert!(first.starts_with("@@ -"), "no file headers, {first:?}");
    for line in patched.patch.lines() {
        assert!(
            line.starts_with("@@")
                || line.starts_with(' ')
                || line.starts_with('+')
                || line.starts_with('-'),
            "unexpected patch row {line:?}",
        );
    }

    // Spread-out edits: many hunks whose total is past the byte budget.
    let wide_old = (0..4_000)
        .map(|line| format!("value {line} padding padding padding\n"))
        .collect::<String>();
    let wide_new = (0..4_000)
        .map(|line| {
            if line % 10 == 0 {
                format!("value {line} padding padding changed\n")
            } else {
                format!("value {line} padding padding padding\n")
            }
        })
        .collect::<String>();
    let wide = compute_patch(
        &wide_old,
        &wide_new,
        Instant::now() + Duration::from_secs(5),
    );
    assert!(wide.truncated, "the patch was cut");
    assert!(wide.patch.len() <= PATCH_MAX_BYTES);
    assert_eq!(wide.stats.added, 400, "stats cover the whole diff");
    assert_eq!(wide.stats.removed, 400);
    assert_eq!(wide.stats.hunks, 400);
    assert_eq!(wide.stats.new_lines, 4_000);
    assert!(
        wide.patch.ends_with('\n')
            && wide
                .patch
                .lines()
                .last()
                .is_some_and(|line| !line.starts_with("@@")),
        "the cut lands on a hunk boundary, never on a bare header",
    );

    let removed = all_removed_patch("alpha\nbeta\n");
    assert_eq!(removed.patch, "@@ -1,2 +0,0 @@\n-alpha\n-beta\n");
    assert_eq!(removed.stats.removed, 2);
    assert_eq!(removed.stats.new_lines, 0);

    let added = all_added_patch("alpha\nbeta\n");
    assert_eq!(added.patch, "@@ -0,0 +1,2 @@\n+alpha\n+beta\n");
    assert_eq!(added.stats.added, 2);
    assert_eq!(added.stats.new_lines, 2);
    assert!(all_added_patch("").patch.is_empty());
}

#[test]
fn read_detects_mid_write() {
    let root = tempdir().expect("tempdir");
    let path = root.path().join("main.ts");

    fs::write(&path, "const answer = 42;\n").expect("write text");
    match read_settled(&path, true, false) {
        ReadOutcome::Text { content, bytes, .. } => {
            assert_eq!(content, "const answer = 42;\n");
            assert_eq!(bytes, 19);
        }
        other => panic!("expected settled text, got {other:?}"),
    }

    // A file that grew (or was re-stamped) between the stat and the re-stat.
    let before = FileStamp {
        len: 19,
        mtime_ms: 1_000,
    };
    assert!(stamp_moved(
        &before,
        &FileStamp {
            len: 40,
            mtime_ms: 1_000
        },
        19
    ));
    assert!(stamp_moved(
        &before,
        &FileStamp {
            len: 19,
            mtime_ms: 1_050
        },
        19
    ));
    assert!(
        stamp_moved(&before, &before, 12),
        "a short read is a moving file"
    );
    assert!(!stamp_moved(&before, &before, 19));

    fs::write(&path, "").expect("truncate");
    assert_eq!(
        read_settled(&path, true, false),
        ReadOutcome::Unsettled,
        "empty against a non-empty baseline is a truncate-then-write",
    );
    match read_settled(&path, false, false) {
        ReadOutcome::Text { content, bytes, .. } => {
            assert!(content.is_empty());
            assert_eq!(bytes, 0);
        }
        other => panic!("expected an honestly empty file, got {other:?}"),
    }

    fs::write(&path, b"abc\0def").expect("write nul");
    assert!(matches!(
        read_settled(&path, false, false),
        ReadOutcome::Opaque {
            reason: OpaqueReason::Binary,
            ..
        },
    ));

    fs::write(&path, [0xff, 0xfe]).expect("write invalid utf8");
    assert!(matches!(
        read_settled(&path, false, false),
        ReadOutcome::Opaque {
            reason: OpaqueReason::Binary,
            ..
        },
    ));

    fs::remove_file(&path).expect("remove");
    assert_eq!(read_settled(&path, false, false), ReadOutcome::Missing);
    assert!(
        matches!(
            read_settled(root.path(), false, false),
            ReadOutcome::Opaque {
                reason: OpaqueReason::Unreadable,
                ..
            },
        ),
        "a directory is not readable text",
    );
}

/// The final attempt after `MAX_RESETTLES` publishes what is on disk instead of
/// re-arming forever: the same file the settled read calls `Unsettled` comes
/// back as `Text`.
#[test]
fn accept_moving_reads_a_file_the_settled_read_holds_back() {
    let root = tempdir().expect("tempdir");
    let path = root.path().join("main.ts");
    fs::write(&path, "").expect("truncate half of a truncate-then-write");

    assert_eq!(
        read_settled(&path, true, false),
        ReadOutcome::Unsettled,
        "the settled read holds an empty file back against a non-empty baseline",
    );
    match read_settled(&path, true, true) {
        ReadOutcome::Text { content, bytes, .. } => {
            assert!(content.is_empty());
            assert_eq!(bytes, 0);
        }
        other => panic!("the forced read publishes what is on disk, got {other:?}"),
    }
}

/// A single hunk cut at `PATCH_MAX_BYTES` still parses: its header counts the
/// rows the patch actually carries, never the rows that were dropped, while
/// `stats` keeps the full totals.
#[test]
fn truncated_single_hunk_header_matches_its_body() {
    let lines: u32 = 1_000;
    let text = (0..lines)
        .map(|_| format!("{}\n", "x".repeat(200)))
        .collect::<String>();

    let removed = all_removed_patch(&text);
    assert!(removed.truncated && removed.patch.len() <= PATCH_MAX_BYTES);
    let rows = removed
        .patch
        .lines()
        .filter(|row| row.starts_with('-'))
        .count();
    assert!(rows > 0 && (rows as u32) < lines, "the body was cut");
    assert_eq!(
        removed.patch.lines().next().expect("a header"),
        format!("@@ -1,{rows} +0,0 @@"),
    );
    assert_eq!(removed.stats.removed, lines, "stats cover the whole file");

    let added = all_added_patch(&text);
    let rows = added
        .patch
        .lines()
        .filter(|row| row.starts_with('+'))
        .count();
    assert_eq!(
        added.patch.lines().next().expect("a header"),
        format!("@@ -0,0 +1,{rows} @@"),
    );
    assert_eq!(added.stats.added, lines);
}
