// Path: crates/im_agent/src/server/socket_writer_tests.rs
// Description: A lagged relay makes the socket writer close the client's connection with 1013

use std::time::Duration;

use futures_util::StreamExt;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::Message;

use super::{spawn_socket_writer, WriterEnd, RELAY_LAGGED_CLOSE_REASON};
use crate::logging::{LogConfig, LogLevel, Logger};
use crate::protocol::{AgentErrorEvent, AgentEvent};
use crate::server::EventBus;

#[tokio::test]
async fn a_lagged_relay_closes_the_client_socket() {
    let dir = tempfile::tempdir().expect("tempdir");
    let logger = Logger::init(LogConfig {
        log_dir: dir.path().join("logs"),
        file_stem: "agent_test",
        min_level: LogLevel::Error,
        emit_stdio: false,
    })
    .await
    .expect("logger");
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let client = tokio::spawn(async move {
        let (socket, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}"))
            .await
            .expect("client connects");
        socket
    });
    let (tcp, _) = listener.accept().await.expect("accept");
    let server = tokio_tungstenite::accept_async(tcp).await.expect("upgrade");
    let mut client = client.await.expect("client task");

    let bus = EventBus::new(4);
    let (relay, _task) = bus.relay(logger.clone(), "peer".to_string(), "lagged");
    for index in 0..10 {
        let event = AgentErrorEvent::new("test", format!("event {index}"), None);
        bus.broadcast_event(AgentEvent::Error(event));
    }
    let (sink, _stream) = server.split();
    let (_response_tx, response_rx) = mpsc::unbounded_channel();
    let writer = spawn_socket_writer(sink, response_rx, relay, logger, "peer".to_string());

    let frame = timeout(Duration::from_secs(2), client.next())
        .await
        .expect("the client hears from the server")
        .expect("a frame")
        .expect("a readable frame");
    let Message::Close(Some(close)) = frame else {
        panic!("expected a close frame, got {frame:?}");
    };
    assert_eq!(close.code, CloseCode::Again);
    assert_eq!(close.reason, RELAY_LAGGED_CLOSE_REASON);
    let end = timeout(Duration::from_secs(2), writer)
        .await
        .expect("the writer ends")
        .expect("writer task");
    assert_eq!(end, WriterEnd::RelayClosed);
}
