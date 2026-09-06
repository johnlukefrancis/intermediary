// Path: crates/im_agent/src/repos/delta/delta_resolve_text_tests.rs
// Description: Text-arm tests - the zero-length first-sighting rule and the READ_DEADLINE bound on the permit wait

use std::sync::Arc;

use tokio::sync::Semaphore;

use crate::protocol::{DeltaBaseline, DeltaPayload, OpaqueReason};

use super::delta_resolve::Resolution;
use super::delta_resolve_text::settled_empty_against_index;
use super::delta_test_support::{change, change_with_op, text, text_of, Harness, RecordingReads};
use super::PendingOp;

/// The zero-length rule, stated on its inputs: only a first sighting that
/// settled empty against a NON-EMPTY index is held back.
#[test]
fn the_zero_length_rule_needs_an_empty_read_against_a_non_empty_index() {
    assert!(settled_empty_against_index(
        DeltaBaseline::Index,
        Some("old\n"),
        ""
    ));
    assert!(
        !settled_empty_against_index(DeltaBaseline::Index, Some(""), ""),
        "an empty index is not a baseline the empty read could falsify"
    );
    assert!(
        !settled_empty_against_index(DeltaBaseline::Index, Some("old\n"), "new\n"),
        "content is content"
    );
    assert!(
        !settled_empty_against_index(DeltaBaseline::PreviousSighting, Some("old\n"), ""),
        "a cached baseline makes an empty read an honest emptying"
    );
    assert!(!settled_empty_against_index(DeltaBaseline::None, None, ""));
}

/// A first sighting that settled empty against a non-empty index blob is
/// dropped: no card, and the cache is left untouched, so the next sighting is
/// still a first sighting that diffs against the index.
#[tokio::test]
async fn an_empty_first_sighting_against_a_non_empty_index_prints_nothing() {
    let reads = RecordingReads::new(Some("old\n"), [text("")]);
    let mut harness = Harness::new(Arc::clone(&reads));
    let mut change = change_with_op(PendingOp::Add);

    assert!(matches!(
        harness.resolve(&mut change).await,
        Resolution::Drop
    ));
    assert_eq!(
        reads.log(),
        vec!["index", "worktree"],
        "both sides were read"
    );
    assert!(
        harness.cache.get("src/main.rs").is_none(),
        "no baseline is stored for a card that never printed"
    );
}

/// The same empty read against an EMPTY index still emits: nothing was
/// falsified, and the zero-stat `VS INDEX` card is the honest one.
#[tokio::test]
async fn an_empty_first_sighting_against_an_empty_index_still_emits() {
    let reads = RecordingReads::new(Some(""), [text("")]);
    let mut harness = Harness::new(Arc::clone(&reads));
    let mut change = change_with_op(PendingOp::Add);

    let (baseline, patch, added, removed) = text_of(harness.resolve(&mut change).await);
    assert_eq!(baseline, DeltaBaseline::Index);
    assert_eq!((added, removed), (0, 0));
    assert!(patch.is_empty());
    assert_eq!(harness.cache.get("src/main.rs").as_deref(), Some(""));
}

/// With both process-wide permits held, the wait is bounded by
/// `READ_DEADLINE` (real time: this test takes one deadline): the delta
/// degrades to an opaque card, the baseline is evicted, and no read was
/// attempted - the worker is free for its next change instead of parked
/// behind the stall.
#[tokio::test]
async fn a_permit_wait_past_read_deadline_degrades_to_an_opaque_card() {
    let reads = RecordingReads::new(Some("old\n"), [text("new\n")]);
    let permits = Arc::new(Semaphore::new(2));
    let held = Arc::clone(&permits)
        .acquire_many_owned(2)
        .await
        .expect("hold every permit");
    let mut harness = Harness::with_permits(Arc::clone(&reads), permits);
    harness
        .cache
        .insert("src/main.rs".to_string(), Arc::from("cached\n"));
    let mut change = change();

    match harness.resolve(&mut change).await {
        Resolution::Emit {
            payload: DeltaPayload::Opaque { bytes, reason },
            failure,
            ..
        } => {
            assert_eq!((bytes, reason), (0, OpaqueReason::Unreadable));
            assert_eq!(
                failure.as_deref(),
                Some("permit wait exceeded READ_DEADLINE")
            );
        }
        Resolution::Emit { payload, .. } => panic!("expected an opaque card, got {payload:?}"),
        Resolution::Resettle | Resolution::Drop => panic!("an expired wait still emits"),
    }
    assert!(reads.log().is_empty(), "no read ran without a permit");
    assert!(
        harness.cache.get("src/main.rs").is_none(),
        "the baseline is evicted like every other READ_DEADLINE expiry"
    );

    drop(held);
    let (baseline, _, added, removed) = text_of(harness.resolve(&mut change).await);
    assert_eq!(
        baseline,
        DeltaBaseline::Index,
        "the next sighting starts over"
    );
    assert_eq!((added, removed), (1, 1));
}
