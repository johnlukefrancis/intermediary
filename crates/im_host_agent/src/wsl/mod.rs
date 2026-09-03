// Path: crates/im_host_agent/src/wsl/mod.rs
// Description: WSL backend client module exports

mod wsl_backend_client;
mod wsl_backend_connection;
mod wsl_backend_messages;

/// The request loop's own message type, so a shutdown-drain test can stand in
/// for the loop and script the answers a real backend would give.
#[cfg(test)]
pub(crate) use wsl_backend_client::RequestLoopMessage;
pub use wsl_backend_client::{ForwardedWslResponse, WslBackendClient};
