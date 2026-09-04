// Path: src-tauri/src/lib/terminal/session_spawn_tests.rs
// Description: Lifecycle oracle of a spawned session on the Linux toolchain: bytes then exit frame, and the console-first close

use super::session_spawn::{spawn_session, SpawnSpec};
use crate::terminal::frames::{CloseOutcome, CloseReason};
use crate::terminal::registry::TerminalRegistry;
use crate::terminal::shell::TerminalCommand;
use std::ffi::OsString;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tauri::ipc::{Channel, InvokeResponseBody};

type Frames = Arc<Mutex<Vec<InvokeResponseBody>>>;

fn spawn_sh(registry: &TerminalRegistry, id: &str, script: &str) -> Frames {
    let frames: Frames = Arc::default();
    let sink = frames.clone();
    let channel = Channel::new(move |body| {
        sink.lock().expect("frames").push(body);
        Ok(())
    });
    let command = TerminalCommand {
        program: "sh".into(),
        args: vec![OsString::from("-c"), OsString::from(script)],
        cwd: std::env::current_dir().expect("cwd"),
        env: std::env::vars_os().collect(),
    };
    let transaction = registry.admit(id, 0).expect("admit");
    let spec = SpawnSpec {
        session_id: id.to_string(),
        command,
        cols: 80,
        rows: 24,
        channel,
    };
    spawn_session(registry, &transaction, spec).expect("spawn");
    frames
}

fn wait_until_empty(registry: &TerminalRegistry) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while registry.session_count().expect("count") > 0 {
        assert!(Instant::now() < deadline, "session never left the registry");
        thread::sleep(Duration::from_millis(20));
    }
}

/// The lifecycle end to end: bytes, then the exit frame, then the session
/// leaves the registry on its own.
#[test]
fn a_child_that_exits_sends_bytes_then_the_exit_frame() {
    let registry = TerminalRegistry::default();
    let frames = spawn_sh(&registry, "exit-3", "printf hello; exit 3");
    wait_until_empty(&registry);

    let frames = frames.lock().expect("frames");
    let mut bytes = Vec::new();
    let mut exit_json = None;
    for frame in frames.iter() {
        match frame {
            InvokeResponseBody::Raw(chunk) => bytes.extend_from_slice(chunk),
            InvokeResponseBody::Json(json) => exit_json = Some(json.clone()),
        }
    }
    assert!(String::from_utf8_lossy(&bytes).contains("hello"));
    let exit_json = exit_json.expect("exit frame");
    assert!(exit_json.contains(r#""code":3"#), "{exit_json}");
    assert!(exit_json.contains(r#""reason":"childExit""#), "{exit_json}");
    assert!(
        matches!(frames.last(), Some(InvokeResponseBody::Json(_))),
        "the exit frame is the last frame"
    );
}

/// Closing a live child ends it inside the console-first budget and frees the slot.
#[test]
fn closing_a_live_child_ends_it_and_frees_the_slot() {
    let registry = TerminalRegistry::default();
    let _frames = spawn_sh(&registry, "sleeper", "sleep 30");
    let outcome = registry
        .close("sleeper", CloseReason::Closed)
        .expect("close");
    assert!(
        matches!(
            outcome,
            CloseOutcome::Exited { .. } | CloseOutcome::Escalated { .. }
        ),
        "{outcome:?}"
    );
    assert_eq!(registry.session_count().expect("count"), 0);
    assert!(registry.close("sleeper", CloseReason::Closed).is_err());
}

/// App exit owns the same spawned resources and does not return until their
/// joined receipt has released the backend slot.
#[test]
fn app_shutdown_joins_a_live_transaction_before_returning() {
    let registry = TerminalRegistry::default();
    let _frames = spawn_sh(&registry, "exit-sleeper", "sleep 30");
    registry.shutdown_all_blocking().expect("shutdown receipt");
    assert_eq!(registry.session_count().expect("count"), 0);
    assert!(registry.admit("after-exit", 0).is_err());
}
