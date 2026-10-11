// Path: crates/im_agent/src/server/ws_server_tests.rs
// Description: The accept loop frees its port the moment shutdown starts, before any drain

use std::time::Duration;

use tokio::net::TcpListener;
use tokio::time::timeout;

use super::{accept_until_stop, AcceptStop, ListenerRelease};
use crate::logging::{LogConfig, LogLevel, Logger};

async fn quiet_logger(dir: &tempfile::TempDir) -> Logger {
    Logger::init(LogConfig {
        log_dir: dir.path().join("logs"),
        file_stem: "agent_test",
        min_level: LogLevel::Error,
        emit_stdio: false,
    })
    .await
    .expect("logger")
}

/// A sibling test's fork may hold the fd until its exec, so the port must free promptly, not instantly.
async fn assert_nothing_listens(port: u16) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
    while TcpListener::bind(("127.0.0.1", port)).await.is_err() {
        assert!(tokio::time::Instant::now() < deadline, "the released port can be bound by a respawn");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// Below Linux's ephemeral range (32768+), so no parallel test's socket can be handed the freed port.
async fn bound_listener(offset: u16) -> (TcpListener, u16) {
    let port = 20_000 + (std::process::id() % 4_000) as u16 * 2 + offset;
    let listener = TcpListener::bind(("127.0.0.1", port)).await.expect("bind");
    (listener, port)
}

/// The `shutdown` command releases before it drains; a respawn must be able to bind at once.
#[tokio::test]
async fn a_release_frees_the_port_while_the_signal_is_still_pending() {
    let dir = tempfile::tempdir().expect("tempdir");
    let logger = quiet_logger(&dir).await;
    let (listener, port) = bound_listener(0).await;
    let release = ListenerRelease::new();
    let shutdown = std::future::pending::<&'static str>();
    tokio::pin!(shutdown);

    release.request();
    let stop = timeout(
        Duration::from_secs(2),
        accept_until_stop(listener, shutdown.as_mut(), &release, &logger, |_, _| {}),
    )
    .await
    .expect("release ends the accept loop");

    assert_eq!(stop, AcceptStop::Released);
    assert_nothing_listens(port).await;
}

#[tokio::test]
async fn a_signal_frees_the_port_and_names_itself() {
    let dir = tempfile::tempdir().expect("tempdir");
    let logger = quiet_logger(&dir).await;
    let (listener, port) = bound_listener(1).await;
    let release = ListenerRelease::new();
    let shutdown = async { "sigterm" };
    tokio::pin!(shutdown);

    let stop = timeout(
        Duration::from_secs(2),
        accept_until_stop(listener, shutdown.as_mut(), &release, &logger, |_, _| {}),
    )
    .await
    .expect("signal ends the accept loop");

    assert_eq!(stop, AcceptStop::Signal("sigterm"));
    assert_nothing_listens(port).await;
}
