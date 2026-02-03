// Path: crates/im_agent/src/protocol/mod.rs
// Description: WebSocket protocol types for the agent

mod commands;
mod envelopes;
mod responses;

pub use commands::{ClientHelloCommand, SetOptionsCommand, UiCommand};
pub use envelopes::{EnvelopeKind, RequestEnvelope, ResponseEnvelope, ResponseError};
pub use responses::{ClientHelloResult, SetOptionsResult, UiResponse};

#[cfg(test)]
mod tests;
