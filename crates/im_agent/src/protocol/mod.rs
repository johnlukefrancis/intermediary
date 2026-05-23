// Path: crates/im_agent/src/protocol/mod.rs
// Description: WebSocket protocol types for the agent

mod commands;
mod envelopes;
mod events;
mod events_runtime;
mod responses;
mod responses_repo;
mod responses_tr_fleet;

#[cfg(test)]
mod cancel_bundle_tests;

pub use commands::{
    BuildBundleCommand, BundleSelection, CancelBundleBuildCommand, ClientHelloCommand,
    GetRepoTopLevelCommand, GetTrFleetStatusCommand, GlobalExcludes, ListBundlesCommand,
    ListRepoDirectoryCommand, ReadImageFileCommand, ReadTextFileCommand, RefreshCommand,
    SetOptionsCommand, StageFileCommand, TrFleetActionCommand, TrFleetActionPayload,
    TrFleetWatchBackend, UiCommand, WatchRepoCommand,
};
pub use envelopes::{
    EnvelopeKind, EventEnvelope, RequestEnvelope, ResponseEnvelope, ResponseError,
};
pub use events::{
    AgentEvent, BundleBuildProgressEvent, BundleBuiltEvent, FileActivity, FileActivityBucket,
    FileChangeType, FileChangedEvent, FileEntry, FileKind, RepoTopologyChangedEvent, SnapshotEvent,
    StagedInfo,
};
pub use events_runtime::{
    AgentErrorCode, AgentErrorDetails, AgentErrorEvent, WslBackendConnectionStatus,
    WslBackendStatusEvent,
};
pub use responses::{
    BuildBundleResult, BundleInfo, CancelBundleBuildResult, ClientHelloResult, ListBundlesResult,
    ReadImageFileResult, ReadTextFileResult, RefreshResult, SetOptionsResult, StageFileResult,
    UiResponse, WatchRepoResult,
};
pub use responses_repo::{GetRepoTopLevelResult, ListRepoDirectoryResult};
pub use responses_tr_fleet::{
    GetTrFleetStatusResult, TrFleetActionKind, TrFleetActionResult, TrFleetEndpointError,
    TrFleetEndpointErrorCode, TrFleetTargetStatus,
};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tr_fleet_tests;
