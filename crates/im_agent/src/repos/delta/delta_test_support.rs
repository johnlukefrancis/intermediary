// Path: crates/im_agent/src/repos/delta/delta_test_support.rs
// Description: Resolver test seam - a scripted ReadSources recorder, a change fixture, and the harness that runs resolve against them

use std::collections::VecDeque;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use futures_util::future::BoxFuture;
use im_bundle::cancel::BundleCancelToken;
use tokio::sync::Semaphore;

use crate::error::AgentError;
use crate::protocol::{DeltaBaseline, DeltaPayload, FileKind};

use super::delta_reads::ReadSources;
use super::delta_resolve::{resolve, Resolution, ResolveContext};
use super::{BaselineCache, PendingChange, PendingOp, ReadOutcome, DELTA_READ_CONCURRENCY};

/// Records which read ran when, and answers each worktree read from a script.
pub(super) struct RecordingReads {
    log: Mutex<Vec<&'static str>>,
    index: Option<&'static str>,
    settled: Mutex<VecDeque<ReadOutcome>>,
}

impl RecordingReads {
    pub(super) fn new(
        index: Option<&'static str>,
        settled: impl IntoIterator<Item = ReadOutcome>,
    ) -> Arc<Self> {
        Arc::new(Self {
            log: Mutex::new(Vec::new()),
            index,
            settled: Mutex::new(settled.into_iter().collect()),
        })
    }

    pub(super) fn log(&self) -> Vec<&'static str> {
        self.log
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .clone()
    }
}

impl ReadSources for RecordingReads {
    fn index_text<'a>(
        &'a self,
        _root: &'a Path,
        _rel: &'a str,
        _cancel: BundleCancelToken,
    ) -> BoxFuture<'a, Result<Option<String>, AgentError>> {
        self.log
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .push("index");
        Box::pin(async move { Ok(self.index.map(str::to_string)) })
    }

    fn settled_read(&self, _abs: &Path, _expect: bool, _accept_moving: bool) -> ReadOutcome {
        self.log
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .push("worktree");
        self.settled
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .pop_front()
            .expect("a scripted worktree read")
    }
}

pub(super) fn text(content: &str) -> ReadOutcome {
    ReadOutcome::Text {
        content: content.to_string(),
        bytes: content.len() as u64,
        mtime_ms: 1_757_168_000_000,
    }
}

/// A first-attempt modify of `src/main.rs`.
pub(super) fn change() -> PendingChange {
    change_with_op(PendingOp::Modify)
}

pub(super) fn change_with_op(op: PendingOp) -> PendingChange {
    let now = Instant::now();
    PendingChange {
        path: "src/main.rs".to_string(),
        abs_path: Path::new("/repo/src/main.rs").to_path_buf(),
        kind: FileKind::Code,
        op,
        first_seen: now,
        last_seen: now,
        deadline: now,
        folded: 0,
        resettles: 0,
        index_baseline: None,
    }
}

pub(super) struct Harness {
    pub(super) cache: BaselineCache,
    pub(super) permits: Arc<Semaphore>,
    reads: Arc<dyn ReadSources>,
    cancel: BundleCancelToken,
}

impl Harness {
    pub(super) fn new(reads: Arc<RecordingReads>) -> Self {
        Self::with_permits(reads, Arc::new(Semaphore::new(DELTA_READ_CONCURRENCY)))
    }

    /// A harness over the caller's semaphore, so a test can hold every permit.
    pub(super) fn with_permits(reads: Arc<RecordingReads>, permits: Arc<Semaphore>) -> Self {
        Self {
            cache: BaselineCache::new(1024),
            permits,
            reads,
            cancel: BundleCancelToken::new(),
        }
    }

    pub(super) async fn resolve(&mut self, change: &mut PendingChange) -> Resolution {
        let mut context = ResolveContext {
            root: Path::new("/repo"),
            cache: &mut self.cache,
            permits: &self.permits,
            reads: &self.reads,
            cancel: &self.cancel,
            may_spawn: true,
        };
        resolve(&mut context, change).await
    }
}

/// The text payload of an emit as `(baseline, patch, added, removed)`.
pub(super) fn text_of(resolution: Resolution) -> (DeltaBaseline, String, u32, u32) {
    match resolution {
        Resolution::Emit {
            payload:
                DeltaPayload::Text {
                    patch,
                    stats,
                    baseline,
                    ..
                },
            ..
        } => (baseline, patch, stats.added, stats.removed),
        Resolution::Emit { payload, .. } => panic!("expected text, got {payload:?}"),
        Resolution::Resettle => panic!("expected an emit, got a resettle"),
        Resolution::Drop => panic!("expected an emit, got a drop"),
    }
}
