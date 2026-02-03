// Path: crates/im_agent/src/error/agent_error.rs
// Description: AgentError type and mapping to protocol error responses

use serde_json::Value;
use thiserror::Error;

use crate::protocol::ResponseError;

#[derive(Debug, Error)]
#[error("{message}")]
pub struct AgentError {
    code: String,
    message: String,
    details: Option<Value>,
}

impl AgentError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new("INTERNAL_ERROR", message)
    }

    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn details(&self) -> Option<&Value> {
        self.details.as_ref()
    }
}

pub fn to_response_error(err: &AgentError) -> ResponseError {
    ResponseError {
        code: err.code.clone(),
        message: err.message.clone(),
        details: err.details.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_agent_error_to_response_error() {
        let details = serde_json::json!({"field": "value"});
        let err = AgentError::new("TEST_CODE", "Something failed").with_details(details.clone());
        let response = to_response_error(&err);

        assert_eq!(response.code, "TEST_CODE");
        assert_eq!(response.message, "Something failed");
        assert_eq!(response.details, Some(details));
    }
}
