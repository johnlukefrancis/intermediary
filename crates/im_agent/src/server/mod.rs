// Path: crates/im_agent/src/server/mod.rs
// Description: WebSocket server module exports

mod connection;
mod event_bus;
mod handshake_auth;
mod runtime_identity;
pub mod shutdown;
mod ws_server;

pub use event_bus::EventBus;
pub use runtime_identity::{attach_runtime_identity_header, runtime_binary_sha256};
pub use shutdown::{
    drain_source_control, drain_source_control_bounded, finalize_shutdown, schedule_process_exit,
    wait_for_shutdown_signal, DrainOutcome, SHUTDOWN_EMERGENCY_BOUND,
};
pub use ws_server::{run_server, ServerConfig};
