// Path: src-tauri/src/lib/agent/supervisor/websocket_probe.rs
// Description: Blocking websocket auth and origin probes used by the supervisor

use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::thread;
use std::time::Duration;

pub(super) const PROBE_TIMEOUT: Duration = Duration::from_millis(750);
const WS_AUTH_PROBE_ATTEMPTS: usize = 3;
const WS_AUTH_PROBE_RETRY_DELAY: Duration = Duration::from_millis(100);

pub(super) fn probe_websocket_auth_blocking(port: u16, token: &str) -> bool {
    if port == 0 || token.trim().is_empty() {
        return false;
    }

    for attempt in 0..WS_AUTH_PROBE_ATTEMPTS {
        match probe_websocket_auth_once(port, token, None) {
            WebSocketAuthProbe::Authenticated => return true,
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
            WebSocketAuthProbe::Authenticated => return true,
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

    let mut buffer = [0_u8; 256];
    let bytes = match stream.read(&mut buffer) {
        Ok(bytes) if bytes > 0 => bytes,
        _ => return WebSocketAuthProbe::Retryable,
    };

    let response = String::from_utf8_lossy(&buffer[..bytes]);
    match response_status_code(&response) {
        Some(101) => WebSocketAuthProbe::Authenticated,
        Some(401) | Some(403) => WebSocketAuthProbe::Rejected,
        _ => WebSocketAuthProbe::Retryable,
    }
}

fn response_status_code(response: &str) -> Option<u16> {
    let status_line = response.lines().next()?;
    status_line.split_whitespace().nth(1)?.parse::<u16>().ok()
}

enum WebSocketAuthProbe {
    Authenticated,
    Rejected,
    Retryable,
}

#[cfg(test)]
mod tests {
    use super::response_status_code;

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
}
