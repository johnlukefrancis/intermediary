// Path: crates/im_agent/src/server/runtime_identity.rs
// Description: Compute and expose the running agent executable identity during WebSocket handshake

use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use tokio_tungstenite::tungstenite::handshake::server::Response;
use tokio_tungstenite::tungstenite::http::{HeaderName, HeaderValue};

pub const RUNTIME_SHA256_HEADER: &str = "x-intermediary-runtime-sha256";

pub fn runtime_binary_sha256() -> Result<String, String> {
    let path = runtime_executable_path()?;
    file_sha256(&path)
}

pub fn attach_runtime_identity_header(mut response: Response, sha256: &str) -> Response {
    let Ok(value) = HeaderValue::from_str(sha256) else {
        return response;
    };
    response
        .headers_mut()
        .insert(HeaderName::from_static(RUNTIME_SHA256_HEADER), value);
    response
}

fn runtime_executable_path() -> Result<PathBuf, String> {
    #[cfg(target_os = "linux")]
    {
        return Ok(PathBuf::from("/proc/self/exe"));
    }

    #[cfg(not(target_os = "linux"))]
    {
        std::env::current_exe()
            .map_err(|err| format!("Failed to resolve running agent executable: {err}"))
    }
}

fn file_sha256(path: &Path) -> Result<String, String> {
    let mut file = File::open(path)
        .map_err(|err| format!("Failed to open running agent executable for hashing: {err}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|err| format!("Failed to hash running agent executable: {err}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::{attach_runtime_identity_header, file_sha256, RUNTIME_SHA256_HEADER};
    use std::fs;
    use tokio_tungstenite::tungstenite::handshake::server::Response;

    #[test]
    fn hashes_file_bytes_with_sha256() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("agent");
        fs::write(&path, b"agent-bytes").expect("fixture");

        assert_eq!(
            file_sha256(&path).expect("sha256"),
            "2c18a5823d3012a1dd7bee6409d4d05b98dfa47733ac4c22e8161445523c10f0"
        );
    }

    #[test]
    fn attaches_runtime_sha256_to_successful_handshake() {
        let response = Response::default();
        let response = attach_runtime_identity_header(response, &"a".repeat(64));

        assert_eq!(
            response
                .headers()
                .get(RUNTIME_SHA256_HEADER)
                .and_then(|value| value.to_str().ok()),
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        );
    }
}
