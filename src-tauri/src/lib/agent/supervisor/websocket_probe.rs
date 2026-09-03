// Path: src-tauri/src/lib/agent/supervisor/websocket_probe.rs
// Description: Blocking websocket auth and origin probes used by the supervisor

use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::thread;
use std::time::Duration;

pub(super) const PROBE_TIMEOUT: Duration = Duration::from_millis(750);
const WS_AUTH_PROBE_ATTEMPTS: usize = 3;
const WS_AUTH_PROBE_RETRY_DELAY: Duration = Duration::from_millis(100);
const HANDSHAKE_RESPONSE_LIMIT: usize = 4 * 1024;
const RUNTIME_SHA256_HEADER: &str = "x-intermediary-runtime-sha256";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct WebSocketIdentityProbe {
    pub authenticated: bool,
    pub runtime_sha256: Option<String>,
}

impl WebSocketIdentityProbe {
    pub fn matches_runtime(&self, expected_sha256: &str) -> bool {
        self.authenticated && self.runtime_sha256.as_deref() == Some(expected_sha256)
    }
}

pub(super) fn probe_websocket_identity_blocking(port: u16, token: &str) -> WebSocketIdentityProbe {
    if port == 0 || token.trim().is_empty() {
        return WebSocketIdentityProbe::default();
    }

    for attempt in 0..WS_AUTH_PROBE_ATTEMPTS {
        match probe_websocket_auth_once(port, token, None) {
            WebSocketAuthProbe::Authenticated(runtime_sha256) => {
                return WebSocketIdentityProbe {
                    authenticated: true,
                    runtime_sha256,
                };
            }
            WebSocketAuthProbe::Rejected => return WebSocketIdentityProbe::default(),
            WebSocketAuthProbe::Retryable => {
                if attempt + 1 < WS_AUTH_PROBE_ATTEMPTS {
                    thread::sleep(WS_AUTH_PROBE_RETRY_DELAY);
                }
            }
        }
    }

    WebSocketIdentityProbe::default()
}

pub(super) fn probe_websocket_origin_compatibility_blocking(
    port: u16,
    token: &str,
    allowed_origins: &[String],
) -> bool {
    if allowed_origins.is_empty() {
        return true;
    }

    allowed_origins
        .iter()
        .all(|origin| probe_websocket_auth_with_origin_blocking(port, token, Some(origin)))
}

fn probe_websocket_auth_with_origin_blocking(port: u16, token: &str, origin: Option<&str>) -> bool {
    if port == 0 || token.trim().is_empty() {
        return false;
    }

    for attempt in 0..WS_AUTH_PROBE_ATTEMPTS {
        match probe_websocket_auth_once(port, token, origin) {
            WebSocketAuthProbe::Authenticated(_) => return true,
            WebSocketAuthProbe::Rejected => return false,
            WebSocketAuthProbe::Retryable => {
                if attempt + 1 < WS_AUTH_PROBE_ATTEMPTS {
                    thread::sleep(WS_AUTH_PROBE_RETRY_DELAY);
                }
            }
        }
    }

    false
}

fn probe_websocket_auth_once(port: u16, token: &str, origin: Option<&str>) -> WebSocketAuthProbe {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
    let mut stream = match TcpStream::connect_timeout(&addr, PROBE_TIMEOUT) {
        Ok(stream) => stream,
        Err(_) => return WebSocketAuthProbe::Retryable,
    };

    let _ = stream.set_read_timeout(Some(PROBE_TIMEOUT));
    let _ = stream.set_write_timeout(Some(PROBE_TIMEOUT));

    let origin_header = origin
        .map(|value| format!("Origin: {value}\r\n"))
        .unwrap_or_default();
    let request = format!(
        "GET /?token={token} HTTP/1.1\r\n\
         Host: 127.0.0.1:{port}\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Version: 13\r\n\
         Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
         {origin_header}\
         \r\n"
    );

    if stream.write_all(request.as_bytes()).is_err() {
        return WebSocketAuthProbe::Retryable;
    }

    let response = match read_handshake_response(&mut stream) {
        Some(response) => response,
        None => return WebSocketAuthProbe::Retryable,
    };
    match response_status_code(&response) {
        Some(101) => WebSocketAuthProbe::Authenticated(response_header_value(
            &response,
            RUNTIME_SHA256_HEADER,
        )),
        Some(401) | Some(403) => WebSocketAuthProbe::Rejected,
        _ => WebSocketAuthProbe::Retryable,
    }
}

/// Shared with the graceful-shutdown client: both open a raw websocket
/// handshake on the same port and need the same two answers from it.
pub(super) fn read_handshake_response(stream: &mut TcpStream) -> Option<String> {
    let mut bytes: Vec<u8> = Vec::with_capacity(512);
    let mut buffer = [0_u8; 512];
    while bytes.len() < HANDSHAKE_RESPONSE_LIMIT {
        let remaining = HANDSHAKE_RESPONSE_LIMIT - bytes.len();
        let chunk_len = remaining.min(buffer.len());
        let read = stream.read(&mut buffer[..chunk_len]).ok()?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    (!bytes.is_empty()).then(|| String::from_utf8_lossy(&bytes).into_owned())
}

pub(super) fn response_status_code(response: &str) -> Option<u16> {
    let status_line = response.lines().next()?;
    status_line.split_whitespace().nth(1)?.parse::<u16>().ok()
}

fn response_header_value(response: &str, header_name: &str) -> Option<String> {
    response
        .lines()
        .skip(1)
        .take_while(|line| !line.trim().is_empty())
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.trim()
                .eq_ignore_ascii_case(header_name)
                .then(|| value.trim().to_string())
        })
        .filter(|value| !value.is_empty())
}

enum WebSocketAuthProbe {
    Authenticated(Option<String>),
    Rejected,
    Retryable,
}

#[cfg(test)]
mod tests {
    use super::{response_header_value, response_status_code, WebSocketIdentityProbe};

    #[test]
    fn parses_websocket_upgrade_status_line() {
        assert_eq!(
            response_status_code("HTTP/1.1 101 Switching Protocols\r\n"),
            Some(101)
        );
        assert_eq!(
            response_status_code("HTTP/1.1 401 Unauthorized\r\n"),
            Some(401)
        );
        assert_eq!(response_status_code("invalid"), None);
    }

    #[test]
    fn parses_runtime_identity_header_case_insensitively() {
        let response = concat!(
            "HTTP/1.1 101 Switching Protocols\r\n",
            "X-Intermediary-Runtime-Sha256: abc123\r\n",
            "\r\n"
        );
        assert_eq!(
            response_header_value(response, "x-intermediary-runtime-sha256").as_deref(),
            Some("abc123")
        );
    }

    #[test]
    fn authenticated_process_requires_exact_runtime_identity() {
        let probe = WebSocketIdentityProbe {
            authenticated: true,
            runtime_sha256: Some("abc123".to_string()),
        };
        assert!(probe.matches_runtime("abc123"));
        assert!(!probe.matches_runtime("different"));
        assert!(!WebSocketIdentityProbe {
            authenticated: true,
            runtime_sha256: None,
        }
        .matches_runtime("abc123"));
    }
}
