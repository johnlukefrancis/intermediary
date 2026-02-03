// Path: crates/im_agent/src/protocol/mod.rs
// Description: WebSocket protocol types for the agent

mod commands;
mod envelopes;
mod events;
mod responses;

pub use commands::{
    BuildBundleCommand, BundleSelection, ClientHelloCommand, GetRepoTopLevelCommand,
    GlobalExcludes, ListBundlesCommand, RefreshCommand, SetOptionsCommand, StageFileCommand,
    UiCommand, WatchRepoCommand,
};
pub use envelopes::{EnvelopeKind, EventEnvelope, RequestEnvelope, ResponseEnvelope, ResponseError};
pub use events::{
    AgentErrorCode, AgentErrorDetails, AgentErrorEvent, AgentEvent, BundleBuildProgressEvent,
    BundleBuiltEvent, FileChangeType, FileChangedEvent, FileEntry, FileKind, SnapshotEvent,
    StagedInfo,
};
pub use responses::{
    BuildBundleResult, BundleInfo, ClientHelloResult, GetRepoTopLevelResult, ListBundlesResult,
    RefreshResult, SetOptionsResult, StageFileResult, UiResponse, WatchRepoResult,
};

#[cfg(test)]
mod tests;
