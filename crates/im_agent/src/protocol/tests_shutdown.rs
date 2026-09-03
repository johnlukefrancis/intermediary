// Path: crates/im_agent/src/protocol/tests_shutdown.rs
// Description: Wire-shape tests for the shutdown command and its result

use super::{InboundRequestEnvelope, ResponseEnvelope, ShutdownResult, UiCommand, UiResponse};

#[test]
fn shutdown_command_is_a_bare_typed_object() {
    let serialized = serde_json::to_string(&UiCommand::Shutdown).expect("serialize shutdown");
    assert_eq!(serialized, r#"{"type":"shutdown"}"#);

    let parsed: UiCommand = serde_json::from_str(&serialized).expect("parse shutdown");
    assert!(matches!(parsed, UiCommand::Shutdown));
    assert_eq!(parsed.command_type(), "shutdown");
    // Shutdown stops the process, not one repository: it must never be routed
    // through repo-scoped dispatch.
    assert_eq!(parsed.repo_id(), None);
}

#[test]
fn shutdown_request_envelope_roundtrips() {
    let request = InboundRequestEnvelope::request("shutdown-1", UiCommand::Shutdown);
    let serialized = serde_json::to_string(&request).expect("serialize request");
    assert_eq!(
        serialized,
        r#"{"kind":"request","requestId":"shutdown-1","payload":{"type":"shutdown"}}"#
    );

    let parsed: InboundRequestEnvelope =
        serde_json::from_str(&serialized).expect("parse shutdown request");
    match parsed {
        InboundRequestEnvelope::Request {
            request_id,
            payload,
        } => {
            assert_eq!(request_id, "shutdown-1");
            assert!(matches!(*payload, UiCommand::Shutdown));
        }
        InboundRequestEnvelope::Cancel { .. } => panic!("expected a request envelope"),
    }
}

#[test]
fn shutdown_result_roundtrips_with_camel_case_fields() {
    let response = ResponseEnvelope::ok(
        "shutdown-1",
        UiResponse::ShutdownResult(ShutdownResult {
            drained: false,
            active_mutations: 2,
        }),
    );
    let serialized = serde_json::to_string(&response).expect("serialize response");
    assert!(
        serialized.contains(r#""type":"shutdownResult""#),
        "unexpected payload: {serialized}"
    );
    assert!(
        serialized.contains(r#""activeMutations":2"#),
        "unexpected payload: {serialized}"
    );

    let parsed: ResponseEnvelope = serde_json::from_str(&serialized).expect("parse response");
    match parsed {
        ResponseEnvelope::Ok { payload, .. } => match payload {
            UiResponse::ShutdownResult(result) => {
                assert!(!result.drained);
                assert_eq!(result.active_mutations, 2);
            }
            _ => panic!("expected a shutdown result"),
        },
        ResponseEnvelope::Error { .. } => panic!("expected an ok response"),
    }
}
