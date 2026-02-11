// Path: crates/im_agent/src/server/mod.rs
// Description: WebSocket server module exports

mod connection;
mod event_bus;
mod handshake_auth;
mod ws_server;

pub use event_bus::EventBus;
pub use ws_server::{run_server, ServerConfig};
