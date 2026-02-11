// Path: crates/im_agent/src/server/handshake_auth.rs
// Description: WSL-agent websocket handshake token validation utilities

use std::sync::Arc;

use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request};
use tokio_tungstenite::tungstenite::http::StatusCode;

#[derive(Clone)]
pub struct ConnectionHandshakeAuth {
    expected_token: Arc<String>,
}

impl ConnectionHandshakeAuth {
    pub fn new(expected_token: String) -> Self {
        Self {
            expected_token: Arc::new(expected_token),
        }
    }

    pub fn validate_request(&self, request: &Request) -> Result<(), HandshakeRejectReason> {
        let provided_token = extract_query_param(request.uri().query(), "token")
            .ok_or(HandshakeRejectReason::MissingToken)?;
        if provided_token != self.expected_token.as_str() {
            return Err(HandshakeRejectReason::InvalidToken);
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub enum HandshakeRejectReason {
    MissingToken,
    InvalidToken,
}

impl HandshakeRejectReason {
    pub fn as_log_reason(&self) -> &'static str {
        match self {
            Self::MissingToken => "missing_token",
            Self::InvalidToken => "invalid_token",
        }
    }
}

pub fn unauthorized_handshake_response() -> ErrorResponse {
    let mut response = ErrorResponse::new(Some("Unauthorized".to_string()));
    *response.status_mut() = StatusCode::UNAUTHORIZED;
    response
}

fn extract_query_param<'a>(query: Option<&'a str>, key: &str) -> Option<&'a str> {
    let query = query?;
    query.split('&').find_map(|segment| {
        let mut parts = segment.splitn(2, '=');
        let segment_key = parts.next()?;
        let segment_value = parts.next().unwrap_or_default();
        if segment_key == key {
            Some(segment_value)
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{ConnectionHandshakeAuth, HandshakeRejectReason};
    use tokio_tungstenite::tungstenite::handshake::server::Request;

    #[test]
    fn accepts_valid_token() {
        let auth = ConnectionHandshakeAuth::new("token123".to_string());
        let request = request_for_uri("/?token=token123");
        assert!(auth.validate_request(&request).is_ok());
    }

    #[test]
    fn rejects_missing_token() {
        let auth = ConnectionHandshakeAuth::new("token123".to_string());
        let request = request_for_uri("/");
        assert!(matches!(
            auth.validate_request(&request),
            Err(HandshakeRejectReason::MissingToken)
        ));
    }

    #[test]
    fn rejects_invalid_token() {
        let auth = ConnectionHandshakeAuth::new("token123".to_string());
        let request = request_for_uri("/?token=wrong");
        assert!(matches!(
            auth.validate_request(&request),
            Err(HandshakeRejectReason::InvalidToken)
        ));
    }

    fn request_for_uri(uri: &str) -> Request {
        Request::builder()
            .uri(uri)
            .body(())
            .expect("request builder should construct test request")
    }
}
