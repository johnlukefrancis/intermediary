// Path: crates/im_agent/src/repos/delta/delta_resolve_tests.rs
// Description: Per-change decision tests - burst charge, rename baseline move, first-sighting truncate guard, image metadata failures

use std::fs;
use std::sync::Arc;
use std::time::Instant;

use tempfile::tempdir;

use crate::logging::{LogConfig, LogLevel, Logger};
use crate::protocol::{DeltaPayload, FileKind};

use super::delta_budget::{BurstBucket, Charge};
use super::delta_resolve::{expect_nonempty, move_rename_baseline, resolve_image, Resolution};
use super::{
    read_settled, BaselineCache, PendingChange, PendingOp, ReadOutcome, BURST_BUDGET, BURST_WINDOW,
    MAX_RESETTLES,
};

/// The budget no longer looks at the file kind or at whether a baseline is
/// cached: a token is charged for every change that will EMIT, because every
/// emitted delta costs a slot on the 128-slot bus whatever it cost to produce.
#[test]
fn every_emitted_change_costs_a_token() {
    let mut bucket = BurstBucket::new(Instant::now());
    for _ in 0..BURST_BUDGET {
        assert_eq!(bucket.charge(&PendingOp::Modify), Charge::Resolve);
    }

    assert_eq!(bucket.charge(&PendingOp::Add), Charge::Withhold);
    assert_eq!(bucket.charge(&PendingOp::Modify), Charge::Withhold);
    assert_eq!(
        bucket.charge(&PendingOp::Rename {
            from: "src/old.rs".to_string(),
        }),
        Charge::Withhold,
    );
    assert_eq!(
        bucket.charge(&PendingOp::Remove),
        Charge::GoneOnly,
        "the one unbudgeted outcome: a deletion still prints, as a bare Gone",
    );
}

/// A closing window says whether it denied anything, which is what lets the
/// worker publish the counters instead of stranding them until the next delta.
#[tokio::test]
async fn a_window_that_denied_says_so_on_close() {
    let temp = tempdir().expect("tempdir");
    let logger = Logger::init(LogConfig {
        log_dir: temp.path().join("logs"),
        min_level: LogLevel::Warn,
        emit_stdio: false,
    })
    .await
    .expect("logger");
    let start = Instant::now();

    let mut quiet = BurstBucket::new(start);
    assert!(
        !quiet.roll(start + BURST_WINDOW, &logger, "repo-1"),
        "a window that denied nothing closes silently",
    );

    let mut spent = BurstBucket::new(start);
    for _ in 0..BURST_BUDGET {
        assert_eq!(spent.charge(&PendingOp::Modify), Charge::Resolve);
    }
    assert_eq!(spent.charge(&PendingOp::Modify), Charge::Withhold);
    assert!(
        !spent.roll(start + BURST_WINDOW / 2, &logger, "repo-1"),
        "a window still open is not closed",
    );
    assert!(spent.roll(start + BURST_WINDOW, &logger, "repo-1"));
    assert!(
        !spent.roll(start + BURST_WINDOW + BURST_WINDOW, &logger, "repo-1"),
        "the denial count reset with the window",
    );
}

/// The baseline moves with a rename exactly once. A re-settled rename finds
/// nothing left at the source, and a second move would evict the very baseline
/// the first attempt carried across.
#[test]
fn rename_moves_the_baseline_once() {
    let mut cache = BaselineCache::new(1024);
    let text: Arc<str> = Arc::from("fn a() {}\n");
    cache.insert("src/old.rs".to_string(), text);

    move_rename_baseline(&mut cache, Some("src/old.rs"), "src/new.rs", 0);
    assert_eq!(cache.get("src/new.rs").as_deref(), Some("fn a() {}\n"));
    assert!(cache.get("src/old.rs").is_none(), "the source moved away");

    move_rename_baseline(&mut cache, Some("src/old.rs"), "src/new.rs", 1);
    assert_eq!(
        cache.get("src/new.rs").as_deref(),
        Some("fn a() {}\n"),
        "a re-settled rename keeps the baseline it already moved",
    );
}

/// A modify says the file already existed, so an empty read is the truncate
/// half of a truncate-then-write even on a first sighting where no baseline
/// exists to compare against. It re-arms until `MAX_RESETTLES`, then publishes
/// what is on disk rather than holding the card back forever.
#[test]
fn a_first_sighting_modify_holds_an_empty_read_back() {
    assert!(
        expect_nonempty(&PendingOp::Modify, None),
        "an uncached modify still expects content",
    );
    assert!(!expect_nonempty(&PendingOp::Add, None));
    assert!(!expect_nonempty(&PendingOp::Add, Some("")));
    assert!(expect_nonempty(&PendingOp::Add, Some("fn a() {}\n")));

    let root = tempdir().expect("tempdir");
    let path = root.path().join("main.ts");
    fs::write(&path, "").expect("truncate half of a write");
    let uncached_modify = expect_nonempty(&PendingOp::Modify, None);

    for resettles in 0..=MAX_RESETTLES {
        let final_attempt = resettles >= MAX_RESETTLES;
        let outcome = read_settled(&path, uncached_modify, final_attempt);
        if final_attempt {
            match outcome {
                ReadOutcome::Text { content, .. } => assert!(content.is_empty()),
                other => panic!("the final attempt publishes what is on disk, got {other:?}"),
            }
        } else {
            assert_eq!(
                outcome,
                ReadOutcome::Unsettled,
                "attempt {resettles} re-arms"
            );
        }
    }
}

/// An image path always lands in a strip tile: a metadata failure that is not a
/// vanished path still publishes an `Image` payload (zero bytes, the path's mime)
/// with the error as its warn, never an `Opaque` that would print a file card.
#[tokio::test]
async fn an_unreadable_image_still_publishes_an_image_payload() {
    let root = tempdir().expect("tempdir");
    // A regular file where a directory is expected: ENOTDIR, not NotFound
    let blocker = root.path().join("assets");
    fs::write(&blocker, "not a directory").expect("blocker file");
    let now = Instant::now();
    let change = PendingChange {
        path: "assets/shot.png".to_string(),
        abs_path: blocker.join("shot.png"),
        kind: FileKind::Image,
        op: PendingOp::Add,
        first_seen: now,
        last_seen: now,
        deadline: now,
        folded: 0,
        resettles: 0,
    };

    match resolve_image(&change).await {
        Resolution::Emit {
            payload: DeltaPayload::Image { bytes, mime_type },
            failure,
            ..
        } => {
            assert_eq!(bytes, 0);
            assert_eq!(mime_type.as_deref(), Some("image/png"));
            assert!(
                failure.is_some(),
                "the metadata error is still warned about"
            );
        }
        Resolution::Emit { payload, .. } => panic!("expected an Image payload, got {payload:?}"),
        Resolution::Resettle | Resolution::Drop => panic!("an unreadable image still emits"),
    }

    let vanished = PendingChange {
        abs_path: root.path().join("missing").join("shot.png"),
        ..change
    };
    let _ = fs::remove_file(&blocker);
    assert!(
        matches!(resolve_image(&vanished).await, Resolution::Drop),
        "a path that is simply gone still drops",
    );
}
