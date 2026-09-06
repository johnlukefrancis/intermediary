// Path: crates/im_agent/src/protocol/tests_delta.rs
// Description: Wire-shape tests for the fileDelta event and its payload union

use serde_json::json;

use super::events::{AgentEvent, FileKind};
use super::events_delta::{
    DeltaBaseline, DeltaOp, DeltaPayload, DeltaStats, FileDeltaCountersEvent, FileDeltaEvent,
    OpaqueReason,
};

fn event(op: DeltaOp, payload: DeltaPayload) -> FileDeltaEvent {
    FileDeltaEvent {
        repo_id: "repo".to_string(),
        seq: 7,
        path: "src/main.ts".to_string(),
        from_path: None,
        kind: FileKind::Code,
        op,
        mtime: "2026-09-06T14:32:07.000Z".to_string(),
        tracked: None,
        folded: 2,
        withheld: 0,
        dropped: 0,
        payload,
    }
}

fn envelope(event: FileDeltaEvent) -> serde_json::Value {
    serde_json::to_value(AgentEvent::FileDelta(event)).expect("serialize fileDelta")
}

fn round_trip(event: FileDeltaEvent) -> FileDeltaEvent {
    let wire = serde_json::to_string(&AgentEvent::FileDelta(event)).expect("serialize");
    match serde_json::from_str::<AgentEvent>(&wire).expect("deserialize") {
        AgentEvent::FileDelta(back) => back,
        other => panic!("expected fileDelta, got {other:?}"),
    }
}

#[test]
fn file_delta_round_trips() {
    let text = event(
        DeltaOp::Modify,
        DeltaPayload::Text {
            patch: "@@ -1,2 +1,2 @@\n-old\n+new\n context\n".to_string(),
            stats: DeltaStats {
                added: 1,
                removed: 1,
                hunks: 1,
                new_lines: 2,
            },
            baseline: DeltaBaseline::PreviousSighting,
            truncated: false,
        },
    );
    assert_eq!(
        envelope(text.clone()),
        json!({
            "type": "fileDelta",
            "repoId": "repo",
            "seq": 7,
            "path": "src/main.ts",
            "kind": "code",
            "op": "modify",
            "mtime": "2026-09-06T14:32:07.000Z",
            "folded": 2,
            "withheld": 0,
            "dropped": 0,
            "payload": {
                "kind": "text",
                "patch": "@@ -1,2 +1,2 @@\n-old\n+new\n context\n",
                "stats": { "added": 1, "removed": 1, "hunks": 1, "newLines": 2 },
                "baseline": "previousSighting",
                "truncated": false,
            },
        }),
        "fromPath and tracked are omitted when None",
    );
    assert_eq!(round_trip(text.clone()), text);

    let image = event(
        DeltaOp::Add,
        DeltaPayload::Image {
            bytes: 4096,
            mime_type: None,
            mtime_ms: 1_757_168_000_000,
        },
    );
    assert_eq!(
        envelope(image.clone())["payload"],
        json!({ "kind": "image", "bytes": 4096, "mimeType": null, "mtimeMs": 1_757_168_000_000_u64 }),
    );
    assert_eq!(round_trip(image.clone()), image);

    let opaque = event(
        DeltaOp::Modify,
        DeltaPayload::Opaque {
            bytes: 900_000,
            reason: OpaqueReason::TooLarge,
        },
    );
    assert_eq!(
        envelope(opaque.clone())["payload"],
        json!({ "kind": "opaque", "bytes": 900_000, "reason": "tooLarge" }),
    );
    assert_eq!(round_trip(opaque.clone()), opaque);

    let mut gone = event(DeltaOp::Remove, DeltaPayload::Gone);
    gone.from_path = Some("src/old.ts".to_string());
    gone.tracked = Some(true);
    gone.op = DeltaOp::Rename;
    let wire = envelope(gone.clone());
    assert_eq!(wire["payload"], json!({ "kind": "gone" }));
    assert_eq!(wire["fromPath"], json!("src/old.ts"));
    assert_eq!(wire["tracked"], json!(true));
    assert_eq!(wire["op"], json!("rename"));
    assert_eq!(round_trip(gone.clone()), gone);
}

#[test]
fn baseline_and_reason_wire_names_are_camel_case() {
    for (baseline, wire) in [
        (DeltaBaseline::PreviousSighting, "previousSighting"),
        (DeltaBaseline::Index, "index"),
        (DeltaBaseline::None, "none"),
    ] {
        assert_eq!(
            serde_json::to_value(baseline).expect("baseline"),
            json!(wire)
        );
    }
    for (reason, wire) in [
        (OpaqueReason::Binary, "binary"),
        (OpaqueReason::TooLarge, "tooLarge"),
        (OpaqueReason::Unreadable, "unreadable"),
    ] {
        assert_eq!(serde_json::to_value(reason).expect("reason"), json!(wire));
    }
}

/// The standalone carrier for the same counters `FileDeltaEvent` piggybacks;
/// it spends a `seq` of its own so a dropped one is a visible gap.
#[test]
fn file_delta_counters_round_trips() {
    let counters = FileDeltaCountersEvent {
        repo_id: "repo".to_string(),
        seq: 8,
        withheld: 468,
        dropped: 12,
    };
    let wire = serde_json::to_value(AgentEvent::FileDeltaCounters(counters.clone()))
        .expect("serialize fileDeltaCounters");
    assert_eq!(
        wire,
        json!({
            "type": "fileDeltaCounters",
            "repoId": "repo",
            "seq": 8,
            "withheld": 468,
            "dropped": 12,
        }),
    );

    let text =
        serde_json::to_string(&AgentEvent::FileDeltaCounters(counters.clone())).expect("serialize");
    match serde_json::from_str::<AgentEvent>(&text).expect("deserialize") {
        AgentEvent::FileDeltaCounters(back) => assert_eq!(back, counters),
        other => panic!("expected fileDeltaCounters, got {other:?}"),
    }
}
