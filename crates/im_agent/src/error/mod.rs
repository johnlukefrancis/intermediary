// Path: crates/im_agent/src/error/mod.rs
// Description: Error module exports for the agent runtime

mod agent_error;

pub use agent_error::{to_response_error, AgentError};
