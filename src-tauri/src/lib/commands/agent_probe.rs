// Path: src-tauri/src/lib/commands/agent_probe.rs
// Description: Probe local agent port availability for diagnostics

use crate::obs::logging;
use serde::Serialize;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::time::Duration;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentPortProbeResult {
    pub listening: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[tauri::command]
pub async fn probe_agent_port(port: u16) -> Result<AgentPortProbeResult, String> {
    let result = tauri::async_runtime::spawn_blocking(move || probe_port_blocking(port))
        .await
        .map_err(|e| {
            let message = format!("Agent probe task failed: {e}");
            logging::log("error", "agent", "probe_failed", &message);
            message
        })?;

    let detail = match (&result.listening, result.error.as_deref()) {
        (true, _) => format!("port={port} listening=true"),
        (false, Some(error)) => format!("port={port} listening=false error={error}"),
        (false, None) => format!("port={port} listening=false"),
    };

    logging::log(
        if result.listening { "info" } else { "warn" },
        "agent",
        "probe",
        &detail,
    );

    Ok(result)
}

fn probe_port_blocking(port: u16) -> AgentPortProbeResult {
    if port == 0 {
        return AgentPortProbeResult {
            listening: false,
            error: Some("Invalid port 0".to_string()),
        };
    }

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
    let timeout = Duration::from_millis(300);

    match TcpStream::connect_timeout(&addr, timeout) {
        Ok(_) => AgentPortProbeResult {
            listening: true,
            error: None,
        },
        Err(err) => AgentPortProbeResult {
            listening: false,
            error: Some(describe_probe_error(port, &err)),
        },
    }
}

fn describe_probe_error(port: u16, err: &std::io::Error) -> String {
    let detail = match err.kind() {
        std::io::ErrorKind::ConnectionRefused => "Connection refused (nothing listening)".to_string(),
        std::io::ErrorKind::TimedOut => "Timed out".to_string(),
        std::io::ErrorKind::AddrNotAvailable => "Address not available".to_string(),
        std::io::ErrorKind::NetworkUnreachable => "Network unreachable".to_string(),
        _ => err.to_string(),
    };

    format!("127.0.0.1:{port} {detail}")
}
