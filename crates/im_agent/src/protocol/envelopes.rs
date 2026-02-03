// Path: crates/im_agent/src/protocol/envelopes.rs
// Description: Protocol envelope types for request/response messaging

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{UiCommand, UiResponse};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnvelopeKind {
    Request,
    Response,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestEnvelope {
    pub kind: EnvelopeKind,
    pub request_id: String,
    pub payload: UiCommand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum ResponseEnvelope {
    Ok {
        kind: EnvelopeKind,
        #[serde(rename = "requestId")]
        request_id: String,
        payload: UiResponse,
    },
    Error {
        kind: EnvelopeKind,
        #[serde(rename = "requestId")]
        request_id: String,
        error: ResponseError,
    },
}

impl ResponseEnvelope {
    pub fn ok(request_id: impl Into<String>, payload: UiResponse) -> Self {
        Self::Ok {
            kind: EnvelopeKind::Response,
            request_id: request_id.into(),
            payload,
        }
    }

    pub fn error(request_id: impl Into<String>, error: ResponseError) -> Self {
        Self::Error {
            kind: EnvelopeKind::Response,
            request_id: request_id.into(),
            error,
        }
    }
}
