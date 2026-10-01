// Path: src-tauri/src/lib/terminal/session_spawn_tests.rs
// Description: Unix PTY lifecycle checks for output, joined exit, and attached process cleanup

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

#[cfg(target_os = "macos")]
#[test]
fn natural_shell_exit_joins_a_hangup_resistant_background_job() {
    let registry = TerminalRegistry::default();
    let frames = spawn_sh(
        &registry,
        "natural-tree",
        "trap '' HUP; sleep 30 & printf armed; exit 7",
    );
    wait_until_empty(&registry);
    let frames = frames.lock().expect("frames");
    assert!(
        matches!(frames.last(), Some(InvokeResponseBody::Json(json)) if json.contains(r#""code":7"#))
    );
}

#[cfg(target_os = "macos")]
#[test]
fn closing_kills_hangup_resistant_background_job_groups() {
    let registry = TerminalRegistry::default();
    let frames = spawn_sh(
        &registry,
        "stubborn",
        "set -m; trap '' HUP; sleep 30 & printf armed; wait",
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let bytes: Vec<u8> = frames
            .lock()
            .expect("frames")
            .iter()
            .flat_map(|frame| match frame {
                InvokeResponseBody::Raw(bytes) => bytes.clone(),
                _ => Vec::new(),
            })
            .collect();
        if String::from_utf8_lossy(&bytes).contains("armed") {
            break;
        }
        assert!(Instant::now() < deadline, "background job never armed");
        thread::sleep(Duration::from_millis(10));
    }
    let outcome = registry
        .close("stubborn", CloseReason::Closed)
        .expect("close");
    assert!(
        matches!(outcome, CloseOutcome::Escalated { .. }),
        "{outcome:?}"
    );
    assert_eq!(registry.session_count().expect("count"), 0);
}

#[test]
fn closing_input_never_supplies_a_missing_line_terminator() {
    let directory = tempfile::tempdir().expect("marker directory");
    let marker = directory.path().join("unsubmitted");
    let registry = TerminalRegistry::default();
    let frames = spawn_sh(
        &registry,
        "unsubmitted",
        &format!(
            "printf READY; IFS= read -r value; printf '%s' \"$value\" > '{}'",
            marker.display()
        ),
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    while !frames.lock().expect("frames").iter().any(|frame| {
        matches!(frame, InvokeResponseBody::Raw(bytes) if String::from_utf8_lossy(bytes).contains("READY"))
    }) {
        assert!(Instant::now() < deadline, "shell never became ready");
        thread::sleep(Duration::from_millis(10));
    }
    let session = registry.running("unsubmitted").expect("running shell");
    session.write(b"pending input").expect("type without Enter");
    session
        .begin_close(CloseReason::Closed)
        .expect("close input only");
    thread::sleep(Duration::from_millis(200));
    assert!(!marker.exists(), "closing input submitted the pending line");
    registry
        .close("unsubmitted", CloseReason::Closed)
        .expect("finish close");
    assert!(!marker.exists(), "teardown submitted the pending line");
}
