// Path: crates/im_agent/src/protocol/tests_events.rs
// Description: Activity event root identity and staged path wire contracts

use serde_json::json;

use super::{AgentEvent, FileChangeType, FileChangedEvent, FileKind, SnapshotEvent};

#[test]
fn legacy_file_changed_staged_windows_path_alias() {
    let json = json!({
        "type": "fileChanged",
        "repoId": "repo",
        "rootPath": "/repo/root/",
        "path": "src/main.ts",
        "kind": "code",
        "changeType": "change",
        "mtime": "2026-02-07T00:00:00Z",
        "staged": {
            "windowsPath": "C:\\staging\\repo\\src\\main.ts",
            "wslPath": "/mnt/c/staging/repo/src/main.ts",
            "bytesCopied": 128,
            "mtimeMs": 1700000000000_u64
        }
    });

    let event: AgentEvent = serde_json::from_value(json).expect("parse staged fileChanged");
    match event {
        AgentEvent::FileChanged(result) => {
            assert_eq!(result.root_path, "/repo/root/");
            let staged = result.staged.expect("staged payload");
            assert_eq!(staged.host_path, "C:\\staging\\repo\\src\\main.ts");
            assert_eq!(
                staged.wsl_path.as_deref(),
                Some("/mnt/c/staging/repo/src/main.ts")
            );
        }
        _ => panic!("expected FileChanged"),
    }
}

#[test]
fn activity_events_require_and_preserve_root_identity() {
    let root = "C:\\repo\\root\\";
    let changed = FileChangedEvent::new(
        "repo".into(),
        root.into(),
        "src/main.ts".into(),
        FileKind::Code,
        FileChangeType::Change,
        "2026-10-09T00:00:00Z".into(),
        None,
    );
    let events = [
        AgentEvent::FileChanged(changed),
        AgentEvent::Snapshot(SnapshotEvent::new("repo".into(), root.into(), Vec::new())),
    ];
    for event in events {
        let mut wire = serde_json::to_value(event).expect("serialize activity");
        assert_eq!(wire["rootPath"], root);
        let back: AgentEvent = serde_json::from_value(wire.clone()).expect("deserialize activity");
        assert_eq!(
            serde_json::to_value(back).expect("reserialize activity"),
            wire
        );
        wire.as_object_mut()
            .expect("event object")
            .remove("rootPath");
        assert!(
            serde_json::from_value::<AgentEvent>(wire).is_err(),
            "root identity is required"
        );
    }
}
