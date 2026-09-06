// Path: crates/im_agent/src/protocol/events_delta.rs
// Description: Wire types for the fileDelta event - what changed inside one file

use serde::{Deserialize, Serialize};

use super::events::FileKind;

/// What happened to the path, as the delta pipeline resolved it. Distinct from
/// `FileChangeType`: a rename is one op here and two `fileChanged` events there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeltaOp {
    Add,
    Modify,
    Remove,
    Rename,
}

/// What the patch is measured against. The card names it so the reader always
/// knows which "before" they are looking at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeltaBaseline {
    /// The text this agent process last served for the path (baseline cache).
    PreviousSighting,
    /// The Git index blob (`git show :0:./<path>`) - the first sighting of a tracked path.
    Index,
    /// No baseline existed: the payload is an all-added patch.
    None,
}

/// Counted over the FULL diff, even when the patch text was truncated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeltaStats {
    pub added: u32,
    pub removed: u32,
    pub hunks: u32,
    /// Line count of the new text, so the UI can print `NEW FILE - N LINES`.
    pub new_lines: u32,
}

/// Why the agent published no text for a path it can see.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OpaqueReason {
    /// A NUL byte or invalid UTF-8.
    Binary,
    /// Over `MAX_DELTA_FILE_BYTES`.
    TooLarge,
    /// The read failed for any other reason.
    Unreadable,
}

/// The body of the card. `kind`-tagged so the zod mirror can discriminate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DeltaPayload {
    /// Hunks-only unified text (`@@`, ` `, `+`, `-` lines; no file headers).
    #[serde(rename_all = "camelCase")]
    Text {
        patch: String,
        stats: DeltaStats,
        baseline: DeltaBaseline,
        /// The patch was cut at `PATCH_MAX_BYTES`; `stats` still covers the whole diff.
        truncated: bool,
    },
    /// Metadata only - the UI fetches pixels through `readImageFile` under its own size gate.
    #[serde(rename_all = "camelCase")]
    Image {
        bytes: u64,
        /// `None` for image extensions with no supported mime (heic/heif/tiff/tif).
        mime_type: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Opaque { bytes: u64, reason: OpaqueReason },
    /// The path is gone and no content was ever sighted for it.
    Gone,
}

/// One bounded fact: what changed inside one file since the agent's previous
/// sighting of it (or since the index, or nothing at all - `payload.baseline`
/// names which). Additive; `fileChanged` semantics are untouched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDeltaEvent {
    pub repo_id: String,
    /// Strictly increasing per repo per agent process; a gap means the 128-slot
    /// event bus dropped a delta, and a restart at 1 means a new stream.
    pub seq: u64,
    pub path: String,
    /// The previous path of a rename; absent otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_path: Option<String>,
    pub kind: FileKind,
    pub op: DeltaOp,
    pub mtime: String,
    /// Best effort only: the tracked set reloads up to ~1 s behind `.git/index`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracked: Option<bool>,
    /// Re-marks folded into this delta by the settle queue.
    pub folded: u32,
    /// Paths the burst budget withheld since the previous emitted delta.
    pub withheld: u32,
    /// Paths the settle queue dropped at `QUEUE_CAP` since the previous emitted delta.
    pub dropped: u32,
    pub payload: DeltaPayload,
}

/// The withheld/dropped counters on their own, published when the delta queue
/// goes quiet or a burst window closes with nothing left to carry them: without
/// this event the counters would sit stranded in the worker until the next
/// delta happened to be emitted. `FileDeltaEvent` carries the same numbers when
/// a delta comes first - whichever carrier arrives first delivers, and both
/// reset the counters, so the UI can add them up without double counting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDeltaCountersEvent {
    pub repo_id: String,
    /// Paths the burst budget withheld since the previous emitted delta.
    pub withheld: u32,
    /// Paths the settle queue dropped at `QUEUE_CAP` since the previous emitted delta.
    pub dropped: u32,
}
