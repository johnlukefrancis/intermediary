// Path: crates/im_host_agent/src/runtime/tr_fleet_service.rs
// Description: Host-agent TR fleet status polling and recovery action execution

use std::time::Duration;

use futures_util::future::join_all;
use im_agent::error::AgentError;
use im_agent::protocol::{
    GetTrFleetStatusResult, TrFleetActionCommand, TrFleetActionKind, TrFleetActionPayload,
    TrFleetActionResult, TrFleetEndpointError, TrFleetEndpointErrorCode, TrFleetTargetStatus,
    TrFleetWatchBackend,
};
use reqwest::Client;
use serde_json::{json, Value};

const TR_FLEET_PORTS: [u16; 5] = [5601, 5602, 5603, 5604, 5605];
const CONTROL_HEADER_KEY: &str = "x-trdev-control";
const CONTROL_HEADER_VALUE: &str = "1";
const REQUEST_TIMEOUT: Duration = Duration::from_millis(900);
const ACTION_TIMEOUT: Duration = Duration::from_millis(1500);
const INVALID_PORT_ERROR: &str = "TR_FLEET_INVALID_PORT";

pub struct TrFleetService {
    client: Client,
}

impl TrFleetService {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn get_tr_fleet_status(&self) -> GetTrFleetStatusResult {
        let targets = join_all(TR_FLEET_PORTS.into_iter().map(|port| self.fetch_port_status(port)))
            .await;
        GetTrFleetStatusResult { targets }
    }

    pub async fn run_tr_fleet_action(
        &self,
        command: TrFleetActionCommand,
    ) -> Result<TrFleetActionResult, AgentError> {
        let action = ActionRequest::from_command(command);
        self.validate_port(action.port)?;

        let endpoint = format!("http://127.0.0.1:{}{}", action.port, action.path);
        let mut request = self
            .client
            .post(endpoint)
            .timeout(ACTION_TIMEOUT)
            .header(CONTROL_HEADER_KEY, CONTROL_HEADER_VALUE);

        if let Some(body) = action.body {
            request = request.json(&body);
        }

        let response = match request.send().await {
            Ok(response) => response,
            Err(err) => {
                return Ok(TrFleetActionResult {
                    action: action.kind,
                    port: action.port,
                    ok: false,
                    status_code: None,
                    response_body: None,
                    error: Some(classify_transport_error(err)),
                });
            }
        };

        let status = response.status();
        let status_code = Some(status.as_u16());
        let body_text = match response.text().await {
            Ok(text) => text,
            Err(err) => {
                return Ok(TrFleetActionResult {
                    action: action.kind,
                    port: action.port,
                    ok: false,
                    status_code,
                    response_body: None,
                    error: Some(TrFleetEndpointError {
                        code: TrFleetEndpointErrorCode::Unknown,
                        message: format!("Failed to read action response body: {err}"),
                        status_code,
                    }),
                });
            }
        };

        let response_body = if body_text.trim().is_empty() {
            None
        } else {
            match serde_json::from_str::<Value>(&body_text) {
                Ok(value) => Some(value),
                Err(_) => Some(Value::String(body_text.clone())),
            }
        };

        let body_ok = response_body
            .as_ref()
            .and_then(|payload| payload.get("ok"))
            .and_then(Value::as_bool)
            .unwrap_or(status.is_success());
        let ok = status.is_success() && body_ok;

        let error = if ok {
            None
        } else {
            Some(action_error(status_code, response_body.as_ref()))
        };

        Ok(TrFleetActionResult {
            action: action.kind,
            port: action.port,
            ok,
            status_code,
            response_body,
            error,
        })
    }

    async fn fetch_port_status(&self, port: u16) -> TrFleetTargetStatus {
        let base_url = format!("http://127.0.0.1:{port}");
        let status_url = format!("{base_url}/__trdev/status");
        let doctor_url = format!("{base_url}/__trdev/doctor");

        let (status_result, doctor_result) = tokio::join!(
            self.fetch_endpoint_json(&status_url),
            self.fetch_endpoint_json(&doctor_url)
        );

        let (status, status_error) = endpoint_result(status_result);
        let (doctor, doctor_error) = endpoint_result(doctor_result);

        TrFleetTargetStatus {
            port,
            base_url,
            status,
            doctor,
            status_error,
            doctor_error,
            fetched_at_ms: now_ms(),
        }
    }

    async fn fetch_endpoint_json(&self, url: &str) -> Result<Value, TrFleetEndpointError> {
        let response = self
            .client
            .get(url)
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(classify_transport_error)?;

        let status = response.status();
        let status_code = status.as_u16();
        let payload = response
            .json::<Value>()
            .await
            .map_err(|err| TrFleetEndpointError {
                code: TrFleetEndpointErrorCode::InvalidJson,
                message: format!("Failed to parse endpoint JSON: {err}"),
                status_code: Some(status_code),
            })?;

        if status.is_success() {
            return Ok(payload);
        }

        let message = payload
            .get("error")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| format!("HTTP {status_code}"));
        Err(TrFleetEndpointError {
            code: TrFleetEndpointErrorCode::HttpError,
            message,
            status_code: Some(status_code),
        })
    }

    fn validate_port(&self, port: u16) -> Result<(), AgentError> {
        if TR_FLEET_PORTS.contains(&port) {
            return Ok(());
        }

        Err(AgentError::new(
            INVALID_PORT_ERROR,
            format!("Unsupported TR fleet port {port}; expected one of 5601..5605"),
        ))
    }
}

struct ActionRequest {
    kind: TrFleetActionKind,
    port: u16,
    path: &'static str,
    body: Option<Value>,
}

impl ActionRequest {
    fn from_command(command: TrFleetActionCommand) -> Self {
        match command.payload {
            TrFleetActionPayload::Rebuild { port } => Self {
                kind: TrFleetActionKind::Rebuild,
                port,
                path: "/__trdev/rebuild",
                body: None,
            },
            TrFleetActionPayload::RestartWatch { port, backend } => Self {
                kind: TrFleetActionKind::RestartWatch,
                port,
                path: "/__trdev/restart-watch",
                body: Some(restart_watch_body(backend)),
            },
        }
    }
}

fn restart_watch_body(backend: TrFleetWatchBackend) -> Value {
    match backend {
        TrFleetWatchBackend::Auto => json!({}),
        TrFleetWatchBackend::Native => json!({ "backend": "native" }),
        TrFleetWatchBackend::Poll => json!({ "backend": "poll" }),
    }
}

fn endpoint_result(
    result: Result<Value, TrFleetEndpointError>,
) -> (Option<Value>, Option<TrFleetEndpointError>) {
    match result {
        Ok(value) => (Some(value), None),
        Err(err) => (None, Some(err)),
    }
}

fn classify_transport_error(err: reqwest::Error) -> TrFleetEndpointError {
    if err.is_timeout() {
        return TrFleetEndpointError {
            code: TrFleetEndpointErrorCode::Timeout,
            message: format!("Request timed out: {err}"),
            status_code: None,
        };
    }
    if err.is_connect() {
        return TrFleetEndpointError {
            code: TrFleetEndpointErrorCode::Unreachable,
            message: format!("Endpoint unreachable: {err}"),
            status_code: None,
        };
    }
    TrFleetEndpointError {
        code: TrFleetEndpointErrorCode::Unknown,
        message: format!("Transport error: {err}"),
        status_code: None,
    }
}

fn action_error(status_code: Option<u16>, response_body: Option<&Value>) -> TrFleetEndpointError {
    let message = response_body
        .and_then(|payload| payload.get("error"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| "Action failed".to_string());

    TrFleetEndpointError {
        code: TrFleetEndpointErrorCode::HttpError,
        message,
        status_code,
    }
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_millis() as u64,
        Err(_) => 0,
    }
}
