// Path: crates/im_agent/src/server/session_log.rs
// Description: Connection lifecycle logging that keeps request-less sessions (the supervisor's health probe) at debug

use serde_json::json;

use crate::logging::Logger;

/// A session that never sends a request is a liveness probe; its connect, reset and
/// disconnect log at debug, while a session that carried requests logs at info/warn.
pub struct SessionLog {
    logger: Logger,
    peer: String,
    label: &'static str,
    requests: u64,
    error: Option<String>,
}

impl SessionLog {
    /// `label` names the client in messages, e.g. `"Client"` or `"Host-agent client"`.
    pub fn opened(logger: Logger, peer: String, label: &'static str) -> Self {
        logger.debug(
            format!("{label} socket opened"),
            Some(json!({"peer": peer})),
        );
        Self {
            logger,
            peer,
            label,
            requests: 0,
            error: None,
        }
    }

    pub fn note_request(&mut self) {
        if self.requests == 0 {
            self.logger.info(
                format!("{} connected", self.label),
                Some(json!({"peer": self.peer})),
            );
        }
        self.requests = self.requests.saturating_add(1);
    }

    pub fn note_error(&mut self, error: String) {
        self.error = Some(error);
    }

    pub fn is_probe(&self) -> bool {
        self.requests == 0
    }

    pub fn closed(self) {
        let data = json!({"peer": self.peer, "requests": self.requests, "error": self.error});
        let message = format!("{} disconnected", self.label);
        if self.is_probe() {
            self.logger.debug(message, Some(data));
        } else if self.error.is_some() {
            self.logger.warn(message, Some(data));
        } else {
            self.logger.info(message, Some(data));
        }
    }
}
