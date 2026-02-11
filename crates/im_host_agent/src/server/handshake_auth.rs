// Path: crates/im_host_agent/src/server/handshake_auth.rs
// Description: Host-agent websocket handshake token and origin validation utilities

use std::collections::HashSet;
use std::sync::Arc;

use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request};
use tokio_tungstenite::tungstenite::http::header::ORIGIN;
use tokio_tungstenite::tungstenite::http::StatusCode;

#[derive(Clone)]
pub struct ConnectionHandshakeAuth {
    expected_token: Arc<String>,
    allowed_origins: Arc<HashSet<String>>,
}

impl ConnectionHandshakeAuth {
    pub fn new(expected_token: String, allowed_origins: Vec<String>) -> Self {
        let allowed_origins = allowed_origins
            .into_iter()
            .map(|origin| origin.trim().to_string())
            .filter(|origin| !origin.is_empty())
            .collect::<HashSet<String>>();
        Self {
            expected_token: Arc::new(expected_token),
            allowed_origins: Arc::new(allowed_origins),
        }
    }

    pub fn validate_request(&self, request: &Request) -> Result<(), HandshakeRejectReason> {
        let provided_token = extract_query_param(request.uri().query(), "token")
            .ok_or(HandshakeRejectReason::MissingToken)?;
        if provided_token != self.expected_token.as_str() {
            return Err(HandshakeRejectReason::InvalidToken);
        }

        if let Some(origin_value) = request.headers().get(ORIGIN) {
            let origin = origin_value
                .to_str()
                .map_err(|_| HandshakeRejectReason::OriginDenied)?;
            if !self.allowed_origins.contains(origin) {
                return Err(HandshakeRejectReason::OriginDenied);
            }
        }

        Ok(())
    }
}

#[derive(Clone, Copy)]
pub enum HandshakeRejectReason {
    MissingToken,
    InvalidToken,
    OriginDenied,
}

impl HandshakeRejectReason {
    pub fn as_log_reason(&self) -> &'static str {
        match self {
            Self::MissingToken => "missing_token",
            Self::InvalidToken => "invalid_token",
            Self::OriginDenied => "origin_denied",
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
    use tokio_tungstenite::tungstenite::http::header::ORIGIN;

    #[test]
    fn accepts_valid_token_without_origin() {
        let auth = ConnectionHandshakeAuth::new("token123".to_string(), vec![]);
        let request = request_for_uri("/?token=token123", None);
        assert!(auth.validate_request(&request).is_ok());
    }

    #[test]
    fn accepts_valid_token_and_allowlisted_origin() {
        let auth = ConnectionHandshakeAuth::new(
            "token123".to_string(),
            vec!["http://localhost:5173".to_string()],
        );
        let request = request_for_uri("/?token=token123", Some("http://localhost:5173"));
        assert!(auth.validate_request(&request).is_ok());
    }

    #[test]
    fn rejects_missing_token() {
        let auth = ConnectionHandshakeAuth::new("token123".to_string(), vec![]);
        let request = request_for_uri("/", None);
        assert!(matches!(
            auth.validate_request(&request),
            Err(HandshakeRejectReason::MissingToken)
        ));
    }

    #[test]
    fn rejects_invalid_token() {
        let auth = ConnectionHandshakeAuth::new("token123".to_string(), vec![]);
        let request = request_for_uri("/?token=wrong", None);
        assert!(matches!(
            auth.validate_request(&request),
            Err(HandshakeRejectReason::InvalidToken)
        ));
    }

    #[test]
    fn rejects_non_allowlisted_origin() {
        let auth = ConnectionHandshakeAuth::new(
            "token123".to_string(),
            vec!["http://localhost:5173".to_string()],
        );
        let request = request_for_uri("/?token=token123", Some("https://evil.example"));
        assert!(matches!(
            auth.validate_request(&request),
            Err(HandshakeRejectReason::OriginDenied)
        ));
    }

    fn request_for_uri(uri: &str, origin: Option<&str>) -> Request {
        let mut builder = Request::builder().uri(uri);
        if let Some(origin) = origin {
            builder = builder.header(ORIGIN, origin);
        }
        builder
            .body(())
            .expect("request builder should construct test request")
    }
}
