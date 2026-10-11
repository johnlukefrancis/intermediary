// Path: crates/im_agent/src/repos/delta/large_delta_tests.rs
// Description: Large-content tests - per-row cap, fingerprint baselines (diff, budget, deletion, rename-over) and the raised read bound

use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::protocol::{DeltaBaseline, DeltaPayload};

use super::baseline::{fingerprints, split_lines};
use super::delta_build::build_delta;
use super::delta_resolve::Resolution;
use super::delta_resolve_text::move_rename_baseline;
use super::delta_test_support::{change_with_op, text, text_of, Harness, RecordingReads};
use super::unified_patch::compute_patch;
use super::{
    all_added_patch, read_settled, Baseline, BaselineCache, PendingOp, ReadOutcome,
    MAX_DELTA_FILE_BYTES, PATCH_LINE_MAX_BYTES,
};

fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(5)
}

fn prints_of(text: &str) -> Baseline {
    Baseline::Lines(fingerprints(&split_lines(text)).expect("fingerprintable"))
}

/// Above `MAX_DELTA_FILE_BYTES` in 60 long lines, so its prints fit a small test cache.
fn large_text(changed: Option<usize>) -> String {
    (0..60)
        .map(|row| {
            let tag = if Some(row) == changed {
                "edited"
            } else {
                "line"
            };
            format!("{tag} {row} {}\n", "x".repeat(10 * 1024))
        })
        .collect()
}

#[test]
fn one_enormous_changed_line_still_shows_its_start() {
    let old = format!("{{\"a\":\"{}\"}}\n", "1".repeat(200 * 1024));
    let new = format!("{{\"a\":\"{}\"}}\n", "2".repeat(200 * 1024));
    let patched = compute_patch(&old, &new, deadline());

    assert!(patched.truncated);
    assert_eq!((patched.stats.added, patched.stats.removed), (1, 1));
    let rows: Vec<&str> = patched.patch.lines().collect();
    assert_eq!(rows.first().copied(), Some("@@ -1 +1 @@"));
    assert!(rows.iter().all(|row| row.len() <= PATCH_LINE_MAX_BYTES + 1));
    assert!(rows.iter().any(|row| row.starts_with("-{\"a\":\"111")));
    assert!(rows.iter().any(|row| row.starts_with("+{\"a\":\"222")));

    let added = all_added_patch(&new);
    assert!(added.truncated && added.patch.starts_with("@@ -0,0 +1,1 @@\n+{\"a\""));

    let multibyte = format!("{}\n", "é".repeat(PATCH_LINE_MAX_BYTES));
    let capped = all_added_patch(&multibyte);
    assert!(capped
        .patch
        .lines()
        .nth(1)
        .is_some_and(|row| row.len() <= PATCH_LINE_MAX_BYTES + 1));
}

#[test]
fn a_fingerprint_baseline_prints_added_text_and_counts_removed_rows() {
    let old = (0..40)
        .map(|row| format!("row {row}\n"))
        .collect::<String>();
    let new = old
        .replace("row 10\nrow 11\n", "row ten\n")
        .replace("row 30\n", "");
    let built = build_delta(Some(&prints_of(&old)), Arc::from(new.as_str()), deadline());
    let oracle = compute_patch(&old, &new, deadline());

    assert_eq!(built.patch.stats, oracle.stats, "stats are exact");
    assert_eq!((built.patch.stats.added, built.patch.stats.removed), (1, 3));
    assert!(built.patch.truncated, "removed text was not retained");
    assert!(built
        .patch
        .patch
        .contains("@@ -8,8 +8,7 @@ 2 removed lines not retained\n"));
    assert!(built.patch.patch.contains(" row 9\n+row ten\n row 12\n"));
    assert!(built
        .patch
        .patch
        .contains("@@ -28,7 +27,6 @@ 1 removed line not retained\n"));
    assert!(!built.patch.patch.lines().any(|row| row.starts_with('-')));
    assert!(
        matches!(built.retained, Some(Baseline::Text(_))),
        "small content keeps text"
    );
}

#[test]
fn fingerprints_count_against_the_repo_budget() {
    let twenty = prints_of(&"a\n".repeat(20));
    assert_eq!(twenty.bytes(), 160);
    let mut cache = BaselineCache::new(400);
    cache.insert("big.html".to_string(), twenty.clone());
    cache.insert(
        "small.rs".to_string(),
        Baseline::Text(Arc::from("x".repeat(100))),
    );
    cache.insert("other.html".to_string(), twenty);
    assert_eq!(
        cache.bytes(),
        260,
        "the least recently used fingerprints went"
    );
    assert!(cache.get("big.html").is_none());

    cache.insert("huge.html".to_string(), prints_of(&"b\n".repeat(60)));
    assert!(
        cache.get("huge.html").is_none(),
        "prints over the whole budget are refused"
    );
}

#[tokio::test]
async fn a_large_file_keeps_fingerprints_and_diffs_against_them() {
    let first = large_text(None);
    assert!(first.len() as u64 > MAX_DELTA_FILE_BYTES);
    let second = large_text(Some(30));
    let reads = RecordingReads::new(None, [text(&first), text(&second)]);
    let mut harness = Harness::new(Arc::clone(&reads));

    let mut change = change_with_op(PendingOp::Add);
    let (baseline, patch, added, removed) = text_of(harness.resolve(&mut change).await);
    assert_eq!((baseline, added, removed), (DeltaBaseline::None, 60, 0));
    assert!(patch.starts_with("@@ -0,0 +1,") && patch.contains("\n+line 0 xxx"));
    assert!(matches!(
        harness.cache.get("src/main.rs"),
        Some(Baseline::Lines(_))
    ));

    let mut change = change_with_op(PendingOp::Modify);
    let (baseline, patch, added, removed) = text_of(harness.resolve(&mut change).await);
    assert_eq!(
        (baseline, added, removed),
        (DeltaBaseline::PreviousSighting, 1, 1)
    );
    assert!(
        patch.starts_with("@@ -28,7 +28,7 @@ 1 removed line not retained\n"),
        "{}",
        &patch[..80]
    );
    assert!(patch.contains("\n+edited 30 xxx"));
    assert!(!patch.lines().any(|row| row.starts_with('-')));

    let mut change = change_with_op(PendingOp::Remove);
    match harness.resolve(&mut change).await {
        Resolution::Emit {
            payload:
                DeltaPayload::Text {
                    patch,
                    stats,
                    truncated,
                    ..
                },
            ..
        } => {
            assert_eq!(patch, "@@ -1,60 +0,0 @@ 60 removed lines not retained\n");
            assert_eq!((stats.removed, truncated), (60, true));
        }
        _ => panic!("a fingerprinted deletion prints its count"),
    }
}

/// An atomic save renames an unindexed temp over the destination: the
/// destination's own fingerprints stay its baseline.
#[tokio::test]
async fn rename_over_keeps_the_destination_baseline() {
    let mut cache = BaselineCache::new(1024);
    cache.insert("src/main.rs".to_string(), prints_of(&large_text(None)));
    move_rename_baseline(&mut cache, Some("src/main.rs.tmp"), "src/main.rs", 0);
    assert!(cache.get("src/main.rs").is_some());

    let reads = RecordingReads::new(None, [text(&large_text(Some(5)))]);
    let mut harness = Harness::new(Arc::clone(&reads));
    harness
        .cache
        .insert("src/main.rs".to_string(), prints_of(&large_text(None)));
    let mut change = change_with_op(PendingOp::Rename {
        from: "src/main.rs.tmp".to_string(),
    });
    let (baseline, _, added, removed) = text_of(harness.resolve(&mut change).await);
    assert_eq!(
        (baseline, added, removed),
        (DeltaBaseline::PreviousSighting, 1, 1)
    );
    assert_eq!(
        reads.log(),
        vec!["worktree"],
        "no index read for a cached destination"
    );
}

#[test]
fn a_large_text_file_is_read_rather_than_opaque() {
    let root = tempfile::tempdir().expect("tempdir");
    let path = root.path().join("page.html");
    std::fs::write(&path, large_text(None)).expect("write");
    assert!(matches!(
        read_settled(&path, false, false),
        ReadOutcome::Text { .. }
    ));
}
