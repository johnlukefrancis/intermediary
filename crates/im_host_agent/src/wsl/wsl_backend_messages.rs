// Path: crates/im_host_agent/src/wsl/wsl_backend_messages.rs
// Description: WSL-backend message parsing and pending-response helpers

use std::collections::HashMap;

use im_agent::error::AgentError;
use im_agent::logging::Logger;
use im_agent::protocol::{AgentEvent, EventEnvelope, ResponseEnvelope, UiResponse};
use im_agent::server::EventBus;
use tokio::sync::oneshot;

use crate::error_codes::WSL_BACKEND_UNAVAILABLE;

pub(super) fn handle_backend_message(
    text: &str,
    pending: &mut HashMap<String, oneshot::Sender<Result<UiResponse, AgentError>>>,
    event_bus: &EventBus,
    logger: &Logger,
) {
    let value: serde_json::Value = match serde_json::from_str(text) {
        Ok(value) => value,
        Err(err) => {
            logger.warn(
                "Invalid JSON from WSL backend",
                Some(serde_json::json!({"error": err.to_string()})),
            );
            return;
        }
    };

    if value.get("kind").and_then(serde_json::Value::as_str) == Some("event") {
        if let Ok(event_envelope) = serde_json::from_value::<EventEnvelope>(value) {
            let event: AgentEvent = event_envelope.payload;
            event_bus.broadcast_event(event);
        }
        return;
    }

    let response: ResponseEnvelope = match serde_json::from_value(value) {
        Ok(response) => response,
        Err(err) => {
            let message = format!(
                "WSL backend protocol mismatch while parsing response: {err}"
            );
            logger.warn(
                "Unexpected message from WSL backend",
                Some(serde_json::json!({"error": err.to_string(), "action": "failing_pending_requests"})),
            );
            fail_pending_requests(pending, message);
            return;
        }
    };

    match response {
        ResponseEnvelope::Ok {
            request_id,
            payload,
            ..
        } => {
            if let Some(response_tx) = pending.remove(&request_id) {
                let _ = response_tx.send(Ok(payload));
            }
        }
        ResponseEnvelope::Error {
            request_id, error, ..
        } => {
            if let Some(response_tx) = pending.remove(&request_id) {
                let mut mapped = AgentError::new(error.code, error.message);
                if let Some(details) = error.details {
                    mapped = mapped.with_details(details);
                }
                let _ = response_tx.send(Err(mapped));
            }
        }
    }
}

pub(super) fn fail_pending_requests(
    pending: &mut HashMap<String, oneshot::Sender<Result<UiResponse, AgentError>>>,
    message: impl Into<String>,
) {
    let err = wsl_unavailable_error(message.into());
    for (_, response_tx) in pending.drain() {
        let _ = response_tx.send(Err(AgentError::new(err.code(), err.message().to_string())));
    }
}

pub(super) fn wsl_unavailable_error(message: impl Into<String>) -> AgentError {
    AgentError::new(WSL_BACKEND_UNAVAILABLE, message)
}
