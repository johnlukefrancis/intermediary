// Path: crates/im_agent/src/repos/repo_watcher_delta_marks.rs
// Description: How watcher events mark the delta queue - one note per change, one rename per two-path arm

use notify::event::RenameMode;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::protocol::{FileChangeType, FileKind};
use crate::repos::delta::PendingOp;
use crate::repos::repo_watcher_events::EventContext;

/// How long a `RenameMode::From` waits for its `To`. ReadDirectoryChangesW
/// reports the two halves of one rename back to back, so the pair is a couple
/// of milliseconds apart; past this window the `From` was a plain delete.
///
/// Strictly below `SETTLE_WINDOW` (120 ms) on purpose: the `From` half leaves a
/// Remove mark on the settle queue, and only a `To` that arrives before that
/// mark can settle still folds into one Rename. A wider window would let the
/// Remove drain and print a delete card that the matching add then contradicts.
const RENAME_PAIR_WINDOW: Duration = Duration::from_millis(80);

/// Whether `apply_change` marks the delta queue itself. A rename arm passes
/// `Skip` for both halves and marks one rename afterwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeltaIntent {
    Note,
    Skip,
}

/// The unpaired `RenameMode::From` half, waiting for the `To` that follows it
/// on the Windows backend. Sync and IO-free: the notify path locks, swaps and
/// releases, never awaiting under the guard (ADR-009).
pub(crate) struct PendingRename {
    slot: Mutex<Option<(PathBuf, Instant)>>,
}

impl PendingRename {
    pub(crate) fn new() -> Self {
        Self {
            slot: Mutex::new(None),
        }
    }

    fn remember(&self, from_path: &Path, now: Instant) {
        *self.lock() = Some((from_path.to_path_buf(), now));
    }

    /// Takes the remembered source when it arrived inside the window. A stale
    /// entry is taken and discarded, so one unpaired `From` can never pair with
    /// an unrelated `To` later on.
    fn take_within(&self, now: Instant, window: Duration) -> Option<PathBuf> {
        let (path, seen) = self.lock().take()?;
        (now.saturating_duration_since(seen) <= window).then_some(path)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Option<(PathBuf, Instant)>> {
        // Nothing awaits under this guard; recover a poisoned lock rather than
        // take the watcher down with it (ADR-008).
        self.slot.lock().unwrap_or_else(|err| err.into_inner())
    }
}

fn pending_op_for(change_type: FileChangeType) -> PendingOp {
    match change_type {
        FileChangeType::Add => PendingOp::Add,
        FileChangeType::Change => PendingOp::Modify,
        FileChangeType::Unlink => PendingOp::Remove,
    }
}

impl EventContext<'_> {
    /// Sync and IO-free: lock, mutate, nudge (ADR-009).
    pub(super) fn note_delta(
        &self,
        relative: String,
        path: &Path,
        kind: FileKind,
        change_type: FileChangeType,
    ) {
        self.delta.note_change(
            relative,
            path.to_path_buf(),
            kind,
            pending_op_for(change_type),
        );
    }

    /// `fileChanged` unlink for `paths[0]` then add for `paths[1]`, exactly as
    /// before, then one rename mark when the destination is a reported file.
    pub(super) async fn apply_rename_pair(&self, paths: &[PathBuf]) {
        let from_path = paths.first();
        if let Some(from_path) = from_path {
            self.apply_change(from_path, FileChangeType::Unlink, DeltaIntent::Skip)
                .await;
        }
        let Some(to_path) = paths.get(1) else {
            return;
        };
        self.mark_rename_to(to_path, from_path.map(PathBuf::as_path))
            .await;
    }

    /// The `From` half of the Windows two-event rename: the plain remove it has
    /// always been (`fileChanged` unlink plus a Remove mark), remembered so a
    /// `To` inside `RENAME_PAIR_WINDOW` can fold it into one rename.
    pub(super) async fn apply_rename_from(&self, from_path: &Path) {
        self.apply_change(from_path, FileChangeType::Unlink, DeltaIntent::Note)
            .await;
        self.pending_rename.remember(from_path, Instant::now());
    }

    /// The `To` half. With a remembered source this is the second half of one
    /// rename: the unlink already went out at `From` time, so only the add and
    /// the single `note_rename` are owed - and the settle queue folds the
    /// pending Remove of the source into the Rename on the destination.
    /// Unpaired, it is the ordinary add it has always been.
    pub(super) async fn apply_rename_to(&self, to_path: &Path) {
        match self
            .pending_rename
            .take_within(Instant::now(), RENAME_PAIR_WINDOW)
        {
            Some(from_path) => self.mark_rename_to(to_path, Some(&from_path)).await,
            None => {
                self.apply_change(to_path, FileChangeType::Add, DeltaIntent::Note)
                    .await;
            }
        }
    }

    /// `fileChanged` add for the destination, then exactly one delta mark: a
    /// rename when the source is known and inside the root, otherwise a plain
    /// add mark.
    async fn mark_rename_to(&self, to_path: &Path, from_path: Option<&Path>) {
        let Some((to_rel, kind)) = self
            .apply_change(to_path, FileChangeType::Add, DeltaIntent::Skip)
            .await
        else {
            return;
        };
        let to_owned = to_path.to_path_buf();
        match from_path.and_then(|from| self.relative_of(from)) {
            Some(from_rel) => self.delta.note_rename(&from_rel, &to_rel, to_owned, kind),
            None => self
                .delta
                .note_change(to_rel, to_owned, kind, PendingOp::Add),
        }
    }
}

/// A two-path arm still emits `fileChanged` unlink + add (Auto Files depends
/// on both) but marks exactly one rename delta. ReadDirectoryChangesW splits
/// that pair across two consecutive one-path arms instead, so `From` remembers
/// itself and `To` completes the same rename.
pub(super) async fn handle_rename_event(
    context: &EventContext<'_>,
    mode: RenameMode,
    paths: &[PathBuf],
) {
    match mode {
        RenameMode::Both => context.apply_rename_pair(paths).await,
        RenameMode::From => {
            if let Some(from_path) = paths.first() {
                context.apply_rename_from(from_path).await;
            }
        }
        RenameMode::To => {
            if let Some(to_path) = paths.first() {
                context.apply_rename_to(to_path).await;
            }
        }
        RenameMode::Any | RenameMode::Other => {
            if paths.len() >= 2 {
                context.apply_rename_pair(paths).await;
            } else if let Some(path) = paths.first() {
                let change_type = infer_rename_change_type(path).await;
                context
                    .apply_change(path, change_type, DeltaIntent::Note)
                    .await;
            }
        }
    }
}

async fn infer_rename_change_type(path: &Path) -> FileChangeType {
    match tokio::fs::metadata(path).await {
        Ok(_) => FileChangeType::Add,
        Err(_) => FileChangeType::Unlink,
    }
}
