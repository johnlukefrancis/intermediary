// Path: crates/im_agent/src/protocol/mod.rs
// Description: WebSocket protocol types for the agent

mod commands;
mod envelopes;
mod events;
mod responses;
mod responses_tr_fleet;

pub use commands::{
    BuildBundleCommand, BundleSelection, ClientHelloCommand, GetRepoTopLevelCommand,
    GetTrFleetStatusCommand, GlobalExcludes, ListBundlesCommand, RefreshCommand,
    SetOptionsCommand, StageFileCommand, TrFleetActionCommand, TrFleetActionPayload,
    TrFleetWatchBackend, UiCommand, WatchRepoCommand,
};
pub use envelopes::{
    EnvelopeKind, EventEnvelope, RequestEnvelope, ResponseEnvelope, ResponseError,
};
pub use events::{
    AgentErrorCode, AgentErrorDetails, AgentErrorEvent, AgentEvent, BundleBuildProgressEvent,
    BundleBuiltEvent, FileChangeType, FileChangedEvent, FileEntry, FileKind, SnapshotEvent,
    StagedInfo, WslBackendConnectionStatus, WslBackendStatusEvent,
};
pub use responses::{
    BuildBundleResult, BundleInfo, ClientHelloResult, GetRepoTopLevelResult, ListBundlesResult,
    RefreshResult, SetOptionsResult, StageFileResult, UiResponse, WatchRepoResult,
};
pub use responses_tr_fleet::{
    GetTrFleetStatusResult, TrFleetActionKind, TrFleetActionResult, TrFleetEndpointError,
    TrFleetEndpointErrorCode, TrFleetTargetStatus,
};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tr_fleet_tests;
