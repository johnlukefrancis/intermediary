// Path: crates/im_agent/src/repos/delta/delta_order_tests.rs
// Description: Resolver causal-order tests through the ReadSources seam - index blob before worktree, capture reused on re-settle, an unchanged non-empty first sighting still emits

use std::sync::Arc;

use crate::protocol::DeltaBaseline;

use super::delta_resolve::Resolution;
use super::delta_test_support::{change, text, text_of, Harness, RecordingReads};
use super::{ReadOutcome, DELTA_READ_CONCURRENCY};

/// On a first sighting the index blob is fetched BEFORE the worktree is read,
/// so the pair a `VS INDEX` card compares was captured in one causal order.
#[tokio::test]
async fn index_blob_is_read_before_the_worktree_on_a_first_sighting() {
    let reads = RecordingReads::new(Some("old\n"), [text("new\n")]);
    let mut harness = Harness::new(Arc::clone(&reads));
    let mut change = change();

    let (baseline, patch, added, removed) = text_of(harness.resolve(&mut change).await);
    assert_eq!(reads.log(), vec!["index", "worktree"]);
    assert_eq!(baseline, DeltaBaseline::Index);
    assert_eq!((added, removed), (1, 1));
    assert!(patch.contains("-old\n+new\n"), "{patch}");
    assert_eq!(
        change
            .index_baseline
            .as_ref()
            .and_then(|text| text.as_deref()),
        Some("old\n"),
        "the capture stays on the change"
    );
}

/// A re-settle reuses the captured index text rather than reading an index a
/// commit may have moved in between: one index read, two worktree reads.
#[tokio::test]
async fn a_resettle_reuses_the_captured_index_baseline() {
    let reads = RecordingReads::new(Some("old\n"), [ReadOutcome::Unsettled, text("new\n")]);
    let mut harness = Harness::new(Arc::clone(&reads));
    let mut change = change();

    assert!(matches!(
        harness.resolve(&mut change).await,
        Resolution::Resettle
    ));
    assert_eq!(
        change
            .index_baseline
            .as_ref()
            .and_then(|text| text.as_deref()),
        Some("old\n")
    );
    change.resettles += 1;
    let (baseline, _, added, removed) = text_of(harness.resolve(&mut change).await);
    assert_eq!(reads.log(), vec!["index", "worktree", "worktree"]);
    assert_eq!(baseline, DeltaBaseline::Index);
    assert_eq!((added, removed), (1, 1));
}

/// A first sighting whose non-empty settled text equals the index still
/// emits: a zero-stat `VS INDEX` delta that says the path was touched and is
/// unchanged. (The empty case is the zero-length rule, `delta_resolve_text_tests`.)
#[tokio::test]
async fn an_unchanged_first_sighting_still_emits_a_zero_stat_delta() {
    let reads = RecordingReads::new(Some("same\n"), [text("same\n")]);
    let mut harness = Harness::new(Arc::clone(&reads));
    let mut change = change();

    let (baseline, patch, added, removed) = text_of(harness.resolve(&mut change).await);
    assert_eq!(baseline, DeltaBaseline::Index);
    assert_eq!((added, removed), (0, 0));
    assert!(patch.is_empty());
    assert_eq!(
        harness.cache.get("src/main.rs").as_deref(),
        Some("same\n"),
        "the sighting is cached, so the next card says SINCE LAST"
    );
    assert_eq!(
        harness.permits.available_permits(),
        DELTA_READ_CONCURRENCY,
        "every owned permit came back with its blocking job"
    );
}
