// Path: crates/im_agent/src/protocol/mod.rs
// Description: WebSocket protocol types for the agent

mod commands;
mod envelopes;
mod responses;

pub use commands::{
    ClientHelloCommand, GetRepoTopLevelCommand, ListBundlesCommand, SetOptionsCommand,
    StageFileCommand, UiCommand,
};
pub use envelopes::{EnvelopeKind, RequestEnvelope, ResponseEnvelope, ResponseError};
pub use responses::{
    BundleInfo, ClientHelloResult, GetRepoTopLevelResult, ListBundlesResult, SetOptionsResult,
    StageFileResult, UiResponse,
};

#[cfg(test)]
mod tests;
