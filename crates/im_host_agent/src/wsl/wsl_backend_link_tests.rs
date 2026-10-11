// Path: crates/im_host_agent/src/wsl/wsl_backend_link_tests.rs
// Description: Host->WSL link bounds - frame size, connect deadline, ping liveness against silent and live peers

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_util::StreamExt;
use im_agent::logging::{LogConfig, LogLevel, Logger};
use im_agent::server::EventBus;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::time::{timeout, Instant};

use super::super::wsl_backend_connection::run_connected;
use super::{connect_bounded, wsl_backend_ws_config, LinkLimits};

const FAST_LIMITS: LinkLimits = LinkLimits {
    connect: Duration::from_millis(200),
    ping_interval: Duration::from_millis(50),
    silence: Duration::from_millis(200),
    send: Duration::from_millis(200),
};

async fn quiet_logger() -> Logger {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock");
    Logger::init(LogConfig {
        log_dir: std::env::temp_dir().join(format!("im_host_agent_link_{}", stamp.as_nanos())),
        file_stem: "agent_test",
        min_level: LogLevel::Error,
        emit_stdio: false,
    })
    .await
    .expect("logger")
}

async fn listener() -> (TcpListener, String) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("addr").port();
    (listener, format!("ws://127.0.0.1:{port}"))
}

/// Two bounded image-diff blobs are ~32 MiB of base64 in one frame; the 16 MiB default
/// would drop the connection, so this hop stays explicitly configured.
#[test]
fn wsl_backend_frame_bound_admits_a_two_sided_image_diff() {
    let config = wsl_backend_ws_config();
    let two_sides = 32 * 1024 * 1024;

    assert!(config.max_frame_size.expect("frame bound") > two_sides);
    assert!(config.max_message_size.expect("message bound") > two_sides);
}

#[tokio::test]
async fn a_peer_that_accepts_but_never_upgrades_fails_the_connect_in_bound() {
    let (listener, endpoint) = listener().await;
    let holder = tokio::spawn(async move { listener.accept().await.map(|(tcp, _)| tcp) });

    let started = Instant::now();
    let err = connect_bounded(&endpoint, FAST_LIMITS.connect)
        .await
        .expect_err("no upgrade arrives");
    assert!(err.contains("exceeded 200ms"), "{err}");
    assert!(started.elapsed() < Duration::from_secs(2));
    holder.abort();
}

/// Runs the connected loop against a peer that either services its socket (tungstenite
/// answers pings while it reads) or holds it unread, like a half-open zombie.
async fn run_against_peer(peer_reads: bool, budget: Duration) -> Option<()> {
    let logger = quiet_logger().await;
    let (listener, endpoint) = listener().await;
    let peer = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await.expect("accept");
        let socket = tokio_tungstenite::accept_async(tcp).await.expect("upgrade");
        // Split like the WSL agent's connection: only its read half is ever polled.
        let (_sink, mut stream) = socket.split();
        if peer_reads {
            while stream.next().await.is_some() {}
        } else {
            std::future::pending::<()>().await;
        }
    });
    let stream = connect_bounded(&endpoint, Duration::from_secs(2))
        .await
        .expect("connect");
    let (_request_tx, mut request_rx) = mpsc::unbounded_channel();
    let outstanding = Arc::new(Mutex::new(HashSet::new()));
    let bus = EventBus::new(8);
    let ended = timeout(
        budget,
        run_connected(
            stream,
            &mut request_rx,
            &bus,
            &logger,
            1,
            &outstanding,
            FAST_LIMITS,
        ),
    )
    .await
    .ok();
    peer.abort();
    ended
}

#[tokio::test]
async fn a_peer_that_stops_answering_pings_is_dropped_after_the_silence_bound() {
    let started = Instant::now();
    let ended = run_against_peer(false, Duration::from_secs(3)).await;
    assert!(ended.is_some(), "a silent peer must end the connection");
    assert!(started.elapsed() >= FAST_LIMITS.silence);
}

#[tokio::test]
async fn a_peer_whose_read_loop_runs_answers_pings_and_stays_connected() {
    let ended = run_against_peer(true, FAST_LIMITS.silence * 4).await;
    assert!(
        ended.is_none(),
        "an answering peer must outlive several silence bounds"
    );
}
