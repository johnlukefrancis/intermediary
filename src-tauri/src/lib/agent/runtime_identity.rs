// Path: src-tauri/src/lib/agent/runtime_identity.rs
// Description: Bounded SHA-256 identity for packaged and installed agent executables

use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub(super) fn executable_sha256(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|err| {
        format!(
            "Failed to open agent executable for hashing ({}): {err}",
            path.display()
        )
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|err| {
            format!(
                "Failed to hash agent executable ({}): {err}",
                path.display()
            )
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::executable_sha256;
    use std::fs;

    #[test]
    fn hashes_agent_bytes_without_loading_the_file_at_once() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("agent");
        fs::write(&path, b"agent-bytes").expect("fixture");

        assert_eq!(
            executable_sha256(&path).expect("sha256"),
            "2c18a5823d3012a1dd7bee6409d4d05b98dfa47733ac4c22e8161445523c10f0"
        );
    }
}
