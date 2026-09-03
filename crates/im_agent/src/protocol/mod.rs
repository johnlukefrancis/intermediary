// Path: crates/im_agent/src/protocol/mod.rs
// Description: WebSocket protocol types for the agent

mod commands;
mod commands_source_control;
mod commands_tr_fleet;
mod envelopes;
mod events;
mod events_legacy_wire;
mod events_runtime;
mod responses;
mod responses_legacy_wire;
mod responses_repo;
mod responses_source_control;
mod responses_tr_fleet;

#[cfg(test)]
mod cancel_bundle_tests;

pub use commands::{
    BuildBundleCommand, BundleSelection, CancelBundleBuildCommand, ClientHelloCommand,
    GetRepoTopLevelCommand, GlobalExcludes, ListBundlesCommand, ListRepoDirectoryCommand,
    ReadImageFileCommand, ReadTextFileCommand, RefreshCommand, SetOptionsCommand, StageFileCommand,
    UiCommand, WatchRepoCommand,
};
pub use commands_tr_fleet::{
    GetTrFleetStatusCommand, TrFleetActionCommand, TrFleetActionPayload, TrFleetWatchBackend,
};
pub use commands_source_control::{
    SourceControlActionCommand, SourceControlActionKind, SourceControlActionPayload,
    SourceControlArea, SourceControlDiffCommand, SourceControlScope, SourceControlStatusCommand,
};
pub use envelopes::{
    EnvelopeKind, EventEnvelope, InboundRequestEnvelope, RequestEnvelope, ResponseEnvelope,
    ResponseError,
};
pub use events::{
    AgentEvent, BundleBuildProgressEvent, BundleBuiltEvent, FileActivity, FileActivityBucket,
    FileChangeType, FileChangedEvent, FileEntry, FileKind, RepoTopologyChangedEvent, SnapshotEvent,
    SourceControlChangedEvent, StagedInfo,
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
pub use responses_source_control::{
    SourceControlActionResult, SourceControlChange, SourceControlDiffResult, SourceControlEntry,
    SourceControlEntryArea, SourceControlOmitted, SourceControlStatus, SourceControlStatusResult,
};
pub use responses_tr_fleet::{
    GetTrFleetStatusResult, TrFleetActionKind, TrFleetActionResult, TrFleetEndpointError,
    TrFleetEndpointErrorCode, TrFleetTargetStatus,
};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tr_fleet_tests;
