// Path: src-tauri/src/lib/agent/supervisor/graceful_stop/tests.rs
// Description: Ack-parsing/route tests for the graceful host stop against a fake agent socket

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

use super::super::shutdown_ws_client::SHUTDOWN_REQUEST_ID;
use super::super::websocket_frame::{read_frame, OPCODE_TEXT};
use super::{AgentSupervisor, GracefulStopPath};
use std::time::Duration;

/// What the fake agent does after the handshake completes.
enum FakeAgentBehavior {
    /// Answers with a `shutdownResult`, then stops accepting connections —
    /// the process is gone right after it spoke.
    AckThenGone { drained: bool },
    /// Closes the connection without ever answering, then stops accepting —
    /// a crash mid-request, indistinguishable from a clean exit at this
    /// layer.
    CloseWithoutAnswering,
    /// Never answers and keeps the port listening: the process is still
    /// alive, just not responding to this request.
    NeverAnswerStayAlive,
}

/// Starts a listener that speaks just enough of the handshake and shutdown
/// exchange to drive `stop_host_gracefully_bounded`, then behaves as directed.
/// Returns the port; the listener's thread outlives the function.
fn spawn_fake_agent(behavior: FakeAgentBehavior) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake agent");
    let port = listener.local_addr().expect("local addr").port();

    thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        if read_http_handshake(&mut stream).is_none() {
            return;
        }
        let _ = stream.write_all(b"HTTP/1.1 101 Switching Protocols\r\n\r\n");
        let _ = read_frame(&mut stream); // the shutdown request frame

        match behavior {
            FakeAgentBehavior::AckThenGone { drained } => {
                let body = format!(
                    r#"{{"status":"ok","kind":"response","requestId":"{SHUTDOWN_REQUEST_ID}","payload":{{"type":"shutdownResult","drained":{drained},"activeMutations":0}}}}"#
                );
                let _ = stream.write_all(&encode_unmasked_text_frame(&body));
                drop(stream);
                // Gone for good: no further connections are ever accepted.
            }
            FakeAgentBehavior::CloseWithoutAnswering => {
                drop(stream);
            }
            FakeAgentBehavior::NeverAnswerStayAlive => {
                // Hold the connection (and the listener, via this thread's
                // ownership of nothing further to drop) open past the test's
                // short bound.
                thread::sleep(Duration::from_secs(5));
            }
        }
    });

    port
}

fn read_http_handshake(stream: &mut TcpStream) -> Option<()> {
    let mut buf = [0_u8; 1];
    let mut seen = Vec::new();
    loop {
        stream.read_exact(&mut buf).ok()?;
        seen.push(buf[0]);
        if seen.ends_with(b"\r\n\r\n") {
            return Some(());
        }
        if seen.len() > 8192 {
            return None;
        }
    }
}

/// One unmasked final text frame, short or 16-bit-extended length (this
/// module's payloads never approach the 64-bit tier).
fn encode_unmasked_text_frame(payload: &str) -> Vec<u8> {
    let bytes = payload.as_bytes();
    let mut frame = vec![0x80 | OPCODE_TEXT];
    match u16::try_from(bytes.len()) {
        Ok(length) if length < 126 => frame.push(length as u8),
        Ok(length) => {
            frame.push(126);
            frame.extend_from_slice(&length.to_be_bytes());
        }
        Err(_) => unreachable!("test payloads never reach the 64-bit length tier"),
    }
    frame.extend_from_slice(bytes);
    frame
}

fn supervisor_with_backend(port: u16) -> AgentSupervisor {
    let supervisor = AgentSupervisor::new();
    supervisor
        .record_owned_host_backend(port, "token")
        .expect("record backend");
    supervisor
}

#[test]
fn an_explicit_drained_true_ack_is_labeled_drained() {
    let port = spawn_fake_agent(FakeAgentBehavior::AckThenGone { drained: true });
    let supervisor = supervisor_with_backend(port);

    let path = tauri::async_runtime::block_on(
        supervisor.stop_host_gracefully_bounded("test", Duration::from_secs(5)),
    );
    assert_eq!(path, GracefulStopPath::Drained);
}

#[test]
fn a_drained_false_ack_with_the_process_then_gone_is_unknown_not_drained() {
    let port = spawn_fake_agent(FakeAgentBehavior::AckThenGone { drained: false });
    let supervisor = supervisor_with_backend(port);

    let path = tauri::async_runtime::block_on(
        supervisor.stop_host_gracefully_bounded("test", Duration::from_secs(5)),
    );
    assert_eq!(path, GracefulStopPath::Unknown);
}

#[test]
fn no_ack_with_the_process_then_gone_is_unknown_not_drained() {
    let port = spawn_fake_agent(FakeAgentBehavior::CloseWithoutAnswering);
    let supervisor = supervisor_with_backend(port);

    let path = tauri::async_runtime::block_on(
        supervisor.stop_host_gracefully_bounded("test", Duration::from_secs(5)),
    );
    assert_eq!(path, GracefulStopPath::Unknown);
}

#[test]
fn no_ack_with_the_process_still_listening_is_incomplete_at_the_bound() {
    let port = spawn_fake_agent(FakeAgentBehavior::NeverAnswerStayAlive);
    let supervisor = supervisor_with_backend(port);

    let path = tauri::async_runtime::block_on(
        supervisor.stop_host_gracefully_bounded("test", Duration::from_millis(300)),
    );
    assert_eq!(path, GracefulStopPath::Incomplete);
}

#[test]
fn no_recorded_backend_is_not_attempted() {
    let supervisor = AgentSupervisor::new();
    let path = tauri::async_runtime::block_on(
        supervisor.stop_host_gracefully_bounded("test", Duration::from_secs(5)),
    );
    assert_eq!(path, GracefulStopPath::NotAttempted);
}
