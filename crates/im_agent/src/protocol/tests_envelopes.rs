// Path: crates/im_agent/src/protocol/tests_envelopes.rs
// Description: Protocol request, response and cancellation envelope roundtrips

use serde_json::json;

use super::{
    ClientHelloCommand, ClientHelloResult, InboundRequestEnvelope, RequestEnvelope,
    ResponseEnvelope, UiCommand, UiResponse,
};

#[test]
fn request_response_roundtrip() {
    let command = UiCommand::ClientHello(ClientHelloCommand {
        config: json!({"autoStageGlobal": true}),
        staging_host_root: "C:\\staging".to_string(),
        staging_wsl_root: Some("/mnt/c/staging".to_string()),
        auto_stage_on_change: Some(false),
    });

    let request = RequestEnvelope {
        kind: super::EnvelopeKind::Request,
        request_id: "req_1".to_string(),
        payload: command,
    };

    let serialized_request = serde_json::to_string(&request).expect("serialize request");
    let parsed_request: RequestEnvelope =
        serde_json::from_str(&serialized_request).expect("parse request");

    assert_eq!(parsed_request.request_id, "req_1");
    let inbound: InboundRequestEnvelope =
        serde_json::from_str(&serialized_request).expect("parse inbound request");
    assert!(matches!(
        inbound,
        InboundRequestEnvelope::Request { request_id, .. } if request_id == "req_1"
    ));

    let response_payload = UiResponse::ClientHelloResult(ClientHelloResult {
        agent_version: "0.1.0".to_string(),
        watched_repo_ids: vec![],
    });

    let response = ResponseEnvelope::ok("req_1", response_payload);
    let serialized_response = serde_json::to_string(&response).expect("serialize response");
    let parsed_response: ResponseEnvelope =
        serde_json::from_str(&serialized_response).expect("parse response");

    match parsed_response {
        ResponseEnvelope::Ok { request_id, .. } => assert_eq!(request_id, "req_1"),
        ResponseEnvelope::Error { .. } => panic!("expected ok response"),
    }
}

#[test]
fn request_cancellation_roundtrip() {
    let cancel = InboundRequestEnvelope::cancel("req_17");
    let serialized = serde_json::to_string(&cancel).expect("serialize cancellation");
    assert_eq!(serialized, r#"{"kind":"cancel","requestId":"req_17"}"#);

    let parsed: InboundRequestEnvelope =
        serde_json::from_str(&serialized).expect("parse cancellation");
    assert!(matches!(
        parsed,
        InboundRequestEnvelope::Cancel { request_id } if request_id == "req_17"
    ));
}
