// Path: src-tauri/src/lib/agent/supervisor/shutdown_ws_client.rs
// Description: One authenticated shutdown request/response exchange with a managed agent

//! The graceful stop rides the agent's existing authenticated websocket port —
//! no second channel, no new port (ADR-010). This is the whole client: connect
//! with the app's host token, send one `shutdown` request, read until the
//! matching response arrives, and close.

use std::io::Write;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::Value;

use super::websocket_frame::{
    encode_client_text_frame, read_frame, OPCODE_CLOSE, OPCODE_CONTINUATION, OPCODE_PING,
    OPCODE_TEXT,
};
use super::websocket_probe::{read_handshake_response, response_status_code};

pub(super) const SHUTDOWN_REQUEST_ID: &str = "shutdown-1";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// The floor on each socket read: a drain answers when it answers, and the
/// caller's own deadline is the real bound.
const MIN_READ_TIMEOUT: Duration = Duration::from_millis(250);

/// What the agent reported before it started exiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct AgentShutdownAck {
    pub drained: bool,
    pub active_mutations: u32,
}

/// Blocking on purpose: the caller runs it on the blocking pool, never on the
/// UI thread.
pub(super) fn request_agent_shutdown_blocking(
    port: u16,
    token: &str,
    budget: Duration,
) -> Result<AgentShutdownAck, String> {
    let deadline = Instant::now() + budget;
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
    let mut stream = TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT)
        .map_err(|err| format!("Agent shutdown connect failed: {err}"))?;
    let _ = stream.set_write_timeout(Some(CONNECT_TIMEOUT));
    apply_read_deadline(&stream, deadline)?;

    handshake(&mut stream, port, token)?;
    let request = shutdown_request_json();
    stream
        .write_all(&encode_client_text_frame(&request, mask_nonce()))
        .map_err(|err| format!("Agent shutdown request write failed: {err}"))?;

    read_shutdown_ack(&mut stream, deadline)
}

fn handshake(stream: &mut TcpStream, port: u16, token: &str) -> Result<(), String> {
    let request = format!(
        "GET /?token={token} HTTP/1.1\r\n\
         Host: 127.0.0.1:{port}\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Version: 13\r\n\
         Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
         \r\n"
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|err| format!("Agent shutdown handshake write failed: {err}"))?;

    let response = read_handshake_response(stream)
        .ok_or_else(|| "Agent shutdown handshake returned no response".to_string())?;
    match response_status_code(&response) {
        Some(101) => Ok(()),
        Some(status) => Err(format!(
            "Agent shutdown handshake rejected with status {status}"
        )),
        None => Err("Agent shutdown handshake returned an unreadable status".to_string()),
    }
}

/// Written literally, not built from a map: this exact envelope is the frozen
/// contract with both agents, and key order should not depend on how a JSON
/// library happens to be compiled.
pub(super) fn shutdown_request_json() -> String {
    format!(
        r#"{{"kind":"request","requestId":"{SHUTDOWN_REQUEST_ID}","payload":{{"type":"shutdown"}}}}"#
    )
}

/// Reads until our own response arrives. The same socket carries broadcast
/// events, so every other message is skipped rather than mistaken for the
/// answer.
fn read_shutdown_ack(
    stream: &mut TcpStream,
    deadline: Instant,
) -> Result<AgentShutdownAck, String> {
    let mut message = String::new();
    loop {
        apply_read_deadline(stream, deadline)?;
        let frame = read_frame(stream)?;
        match frame.opcode {
            OPCODE_TEXT | OPCODE_CONTINUATION => {
                message.push_str(&String::from_utf8_lossy(&frame.payload));
                if !frame.fin {
                    continue;
                }
                let complete = std::mem::take(&mut message);
                if let Some(ack) = parse_shutdown_ack(&complete)? {
                    return Ok(ack);
                }
            }
            OPCODE_CLOSE => {
                return Err("Agent closed the socket before answering shutdown".to_string());
            }
            OPCODE_PING => {}
            _ => {}
        }
    }
}

/// `Ok(None)` means "not our message" (an event, or another request's
/// response); an error envelope for our request id is a refusal and ends the
/// exchange.
pub(super) fn parse_shutdown_ack(message: &str) -> Result<Option<AgentShutdownAck>, String> {
    let Ok(value) = serde_json::from_str::<Value>(message) else {
        return Ok(None);
    };
    if value.get("requestId").and_then(Value::as_str) != Some(SHUTDOWN_REQUEST_ID) {
        return Ok(None);
    }
    if let Some(error) = value.get("error") {
        let code = error
            .get("code")
            .and_then(Value::as_str)
            .unwrap_or("UNKNOWN");
        let detail = error.get("message").and_then(Value::as_str).unwrap_or("");
        return Err(format!("Agent refused shutdown: {code} {detail}"));
    }

    let payload = value
        .get("payload")
        .ok_or_else(|| "Agent shutdown response carried no payload".to_string())?;
    Ok(Some(AgentShutdownAck {
        drained: payload
            .get("drained")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        active_mutations: payload
            .get("activeMutations")
            .and_then(Value::as_u64)
            .and_then(|count| u32::try_from(count).ok())
            .unwrap_or(0),
    }))
}

fn apply_read_deadline(stream: &TcpStream, deadline: Instant) -> Result<(), String> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err("Agent shutdown exchange exceeded its budget".to_string());
    }
    stream
        .set_read_timeout(Some(remaining.max(MIN_READ_TIMEOUT)))
        .map_err(|err| format!("Agent shutdown read timeout could not be set: {err}"))
}

fn mask_nonce() -> u32 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.subsec_nanos())
        .unwrap_or(0x5a5a_5a5a);
    nanos | 1
}

#[cfg(test)]
mod tests {
    use super::{parse_shutdown_ack, shutdown_request_json, SHUTDOWN_REQUEST_ID};

    #[test]
    fn the_request_is_the_frozen_wire_shape() {
        assert_eq!(
            shutdown_request_json(),
            r#"{"kind":"request","requestId":"shutdown-1","payload":{"type":"shutdown"}}"#
        );
    }

    #[test]
    fn a_shutdown_result_is_read_back() {
        let message = format!(
            r#"{{"status":"ok","kind":"response","requestId":"{SHUTDOWN_REQUEST_ID}","payload":{{"type":"shutdownResult","drained":false,"activeMutations":3}}}}"#
        );
        let ack = parse_shutdown_ack(&message)
            .expect("parse")
            .expect("our response");
        assert!(!ack.drained);
        assert_eq!(ack.active_mutations, 3);
    }

    #[test]
    fn events_and_other_requests_are_skipped() {
        let event = r#"{"kind":"event","payload":{"type":"snapshot"}}"#;
        assert!(parse_shutdown_ack(event).expect("parse").is_none());
        let other = r#"{"status":"ok","kind":"response","requestId":"req_9","payload":{}}"#;
        assert!(parse_shutdown_ack(other).expect("parse").is_none());
    }

    #[test]
    fn an_error_envelope_ends_the_exchange() {
        let refusal = format!(
            r#"{{"status":"error","kind":"response","requestId":"{SHUTDOWN_REQUEST_ID}","error":{{"code":"UNKNOWN_COMMAND","message":"Unsupported command"}}}}"#
        );
        let err = parse_shutdown_ack(&refusal).expect_err("refusal");
        assert!(err.contains("UNKNOWN_COMMAND"), "unexpected: {err}");
    }
}
