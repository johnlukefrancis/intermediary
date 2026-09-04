// Path: src-tauri/src/lib/terminal/session_open.rs
// Description: Opens a pwsh session for a repo root: validation, shell and start-dir resolution, spawn, and the open/open-failed log lines

use super::frames::{TerminalOpenRequest, TerminalOpened};
use super::registry::TerminalRegistry;
use super::session::validate_size;
use super::session_spawn::{spawn_session, SpawnError, SpawnSpec};
use super::{shell, start_dir, windows_build};
use crate::config::types::RepoRoot;
use crate::obs::logging;
use std::time::Instant;
use tauri::ipc::{Channel, InvokeResponseBody};

/// `distro` is the configured WSL distro (already resolved by the caller);
/// it matters only for a native WSL root.
pub fn open_session(
    registry: &TerminalRegistry,
    request: TerminalOpenRequest,
    distro: Option<String>,
    channel: Channel<InvokeResponseBody>,
    page_generation: u32,
) -> Result<TerminalOpened, String> {
    let started = Instant::now();
    let root_kind = match &request.repo_root {
        RepoRoot::Wsl { .. } => "wsl",
        RepoRoot::Host { .. } => "host",
    };
    match open_inner(
        registry,
        &request,
        distro.as_deref(),
        channel,
        page_generation,
    ) {
        Ok(opened) => {
            logging::log(
                "info",
                "terminal",
                "session_open",
                &format!(
                    "id={} pid={} cols={} rows={} root_kind={root_kind} spawn_ms={}",
                    opened.session_id,
                    opened.pid,
                    request.cols,
                    request.rows,
                    started.elapsed().as_millis()
                ),
            );
            Ok(opened)
        }
        Err(err) => {
            logging::log(
                "error",
                "terminal",
                "session_open_failed",
                &format!(
                    "id={} phase={} root_kind={root_kind} error=\"{}\"",
                    request.session_id, err.phase, err.message
                ),
            );
            Err(err.message)
        }
    }
}

fn open_inner(
    registry: &TerminalRegistry,
    request: &TerminalOpenRequest,
    distro: Option<&str>,
    channel: Channel<InvokeResponseBody>,
    page_generation: u32,
) -> Result<TerminalOpened, SpawnError> {
    validate_session_id(&request.session_id).map_err(|err| SpawnError::new("validate", err))?;
    validate_size(request.cols, request.rows).map_err(|err| SpawnError::new("validate", err))?;
    let transaction = registry
        .admit(&request.session_id, page_generation)
        .map_err(|err| SpawnError::new("admission", err))?;
    let result = (|| {
        let pwsh = shell::resolve_pwsh().map_err(|err| SpawnError::new("resolve_pwsh", err))?;
        let start = start_dir::resolve(&request.repo_root, distro)
            .map_err(|err| SpawnError::new("start_dir", err))?;
        let command = shell::build_command(&pwsh, &start);
        let session = spawn_session(
            registry,
            &transaction,
            SpawnSpec {
                session_id: request.session_id.clone(),
                command,
                cols: request.cols,
                rows: request.rows,
                channel,
            },
        )?;
        Ok(TerminalOpened {
            session_id: session.id.clone(),
            pid: session.pid,
            windows_build_number: windows_build::current_build_number(),
            start_dir: start.cwd.to_string_lossy().into_owned(),
            initial_command: start.initial_command,
        })
    })();
    if result.is_err() {
        registry
            .fail_open(&transaction)
            .map_err(|err| SpawnError::new("admission_release", err))?;
    }
    result
}

/// The frontend mints the id; it names threads and log lines, so only the
/// canonical UUID text form is accepted.
pub fn validate_session_id(id: &str) -> Result<(), String> {
    let valid = id.len() == 36
        && id.bytes().enumerate().all(|(index, byte)| match index {
            8 | 13 | 18 | 23 => byte == b'-',
            _ => byte.is_ascii_hexdigit(),
        });
    if valid {
        Ok(())
    } else {
        Err("Terminal session id must be a UUID".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::validate_session_id;

    #[test]
    fn only_the_canonical_uuid_form_is_a_session_id() {
        assert!(validate_session_id("3f2504e0-4f89-11d3-9a0c-0305e82c3301").is_ok());
        assert!(validate_session_id("3F2504E0-4F89-11D3-9A0C-0305E82C3301").is_ok());
        assert!(validate_session_id("3f2504e04f8911d39a0c0305e82c3301").is_err());
        assert!(validate_session_id("3f2504e0-4f89-11d3-9a0c-0305e82c330g").is_err());
        assert!(validate_session_id("../3f2504e0-4f89-11d3-9a0c-0305e82c33").is_err());
        assert!(validate_session_id("").is_err());
    }

    /// Off Windows the open is refused before anything is spawned, with the
    /// reason a user can act on; it never pretends to succeed.
    #[cfg(not(windows))]
    #[test]
    fn off_windows_the_open_is_an_honest_error() {
        use super::open_session;
        use crate::config::types::RepoRoot;
        use crate::terminal::frames::TerminalOpenRequest;
        use crate::terminal::registry::TerminalRegistry;
        use tauri::ipc::Channel;

        let registry = TerminalRegistry::default();
        let request = TerminalOpenRequest {
            session_id: "3f2504e0-4f89-11d3-9a0c-0305e82c3301".to_string(),
            repo_root: RepoRoot::Host {
                path: "/tmp".to_string(),
            },
            cols: 80,
            rows: 24,
        };
        let err = open_session(&registry, request, None, Channel::new(|_| Ok(())), 0)
            .expect_err("no shell off Windows");
        assert!(err.contains("Windows hosts only"), "{err}");
        assert_eq!(registry.session_count().expect("count"), 0);
    }
}
