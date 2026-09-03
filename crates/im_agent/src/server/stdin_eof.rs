// Path: crates/im_agent/src/server/stdin_eof.rs
// Description: The supervisor's stdin pipe as a shutdown owner - EOF on fd 0 is a drain request

//! The supervisor that starts this agent inside WSL holds the write end of its
//! stdin pipe for exactly as long as it intends the agent to live. When that
//! end closes — the supervisor asked for it, or the supervisor itself is gone —
//! fd 0 reaches EOF, and that is the same request SIGTERM and the authenticated
//! `shutdown` command make: drain, then exit. It is the only one of the three
//! that survives the supervisor dying without a chance to send anything.
//!
//! Only a pipe or a socket is an owner. A tty is a developer's terminal, whose
//! closing already arrives as SIGHUP/SIGTERM; `/dev/null` is a script launch and
//! reaches EOF immediately, which would mean "exit now" rather than "the
//! supervisor let go". Neither is claimed, so a terminal or script launch
//! behaves exactly as it did before this owner existed.

use std::io::Read;

use serde_json::json;

use crate::logging::Logger;

/// The reason this owner reports, taking the same drain path as `sigterm`.
pub const STDIN_EOF_REASON: &str = "stdin-eof";

/// Resolves when the supervisor's end of the stdin pipe closes. When fd 0 is
/// not a supervisor pipe this never resolves, so the caller's `select!` is left
/// to SIGTERM and ctrl-c exactly as before.
pub async fn wait_for_stdin_eof(logger: &Logger) -> &'static str {
    if !stdin_is_supervisor_pipe() {
        logger.info(
            "Stdin is not a supervisor pipe; no EOF shutdown owner is installed",
            Some(json!({"stdin": stdin_kind()})),
        );
        std::future::pending::<()>().await;
    }

    logger.info(
        "Watching the supervisor's stdin pipe for EOF",
        Some(json!({"stdin": stdin_kind(), "reason": STDIN_EOF_REASON})),
    );

    // fd 0 has no async owner in this process, and a blocking read parks in the
    // kernel until the writer closes — so it belongs on the blocking pool, never
    // on the runtime that serves the socket loop (ADR-009). The task ends at EOF
    // or with the process; there is nothing here to spin on.
    match tokio::task::spawn_blocking(drain_stdin_to_eof).await {
        Ok(Ok(())) => STDIN_EOF_REASON,
        Ok(Err(err)) => retire_owner(logger, &err.to_string()).await,
        Err(err) => retire_owner(logger, &err.to_string()).await,
    }
}

/// The pipe can no longer be read, so this owner is gone — but a read that
/// failed is not a supervisor that let go, and shutting down on it would end the
/// agent for a reason nobody asked for. Park instead and leave finality to
/// SIGTERM.
async fn retire_owner(logger: &Logger, error: &str) -> &'static str {
    logger.warn(
        "Failed to read the supervisor's stdin pipe; the EOF shutdown owner is retired",
        Some(json!({"error": error})),
    );
    std::future::pending::<&'static str>().await
}

/// Whether fd 0 is a pipe or a socket — the two shapes a supervisor's handle
/// takes. Anything else (a tty, `/dev/null`, a regular file) is not claimed.
pub fn stdin_is_supervisor_pipe() -> bool {
    is_supervisor_pipe_fd(0)
}

#[cfg(unix)]
fn is_supervisor_pipe_fd(fd: std::os::unix::io::RawFd) -> bool {
    matches!(file_type_of(fd), Some(kind) if kind == libc::S_IFIFO || kind == libc::S_IFSOCK)
}

#[cfg(not(unix))]
fn is_supervisor_pipe_fd(_fd: i32) -> bool {
    false
}

#[cfg(unix)]
fn file_type_of(fd: std::os::unix::io::RawFd) -> Option<libc::mode_t> {
    let mut info = std::mem::MaybeUninit::<libc::stat>::zeroed();
    // SAFETY: `fstat` only writes into the fully owned, zeroed `stat` this call
    // provides and reads no Rust memory. An invalid or closed descriptor answers
    // -1 rather than writing anything.
    let rc = unsafe { libc::fstat(fd, info.as_mut_ptr()) };
    if rc != 0 {
        return None;
    }
    // SAFETY: `fstat` returned 0, so it initialised the whole struct.
    let mode = unsafe { info.assume_init() }.st_mode;
    Some(mode & libc::S_IFMT)
}

/// What fd 0 is, for the log line that says whether this owner was installed.
#[cfg(unix)]
fn stdin_kind() -> &'static str {
    match file_type_of(0) {
        Some(kind) if kind == libc::S_IFIFO => "pipe",
        Some(kind) if kind == libc::S_IFSOCK => "socket",
        Some(kind) if kind == libc::S_IFCHR => "tty_or_null",
        Some(kind) if kind == libc::S_IFREG => "file",
        Some(_) => "other",
        None => "unavailable",
    }
}

#[cfg(not(unix))]
fn stdin_kind() -> &'static str {
    "unsupported_platform"
}

/// Consumes whatever the supervisor writes (it writes nothing) until the pipe
/// closes. Bounded by the writer: `read` returns 0 exactly once, at EOF.
fn drain_stdin_to_eof() -> std::io::Result<()> {
    let mut stdin = std::io::stdin().lock();
    let mut scratch = [0u8; 1024];
    loop {
        match stdin.read(&mut scratch) {
            Ok(0) => return Ok(()),
            Ok(_) => {}
            Err(err) if err.kind() == std::io::ErrorKind::Interrupted => {}
            Err(err) => return Err(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::STDIN_EOF_REASON;

    /// The reason string is the contract with the supervisor's logs and with
    /// `finalize_shutdown`, which records it verbatim.
    #[test]
    fn the_eof_reason_names_the_owner() {
        assert_eq!(STDIN_EOF_REASON, "stdin-eof");
    }

    /// The classification is what keeps a terminal or `/dev/null` launch from
    /// drain-and-exiting at startup, so it is tested against real descriptors of
    /// each shape rather than against whatever fd 0 happens to be.
    #[cfg(unix)]
    #[test]
    fn only_a_pipe_or_socket_is_claimed_as_a_supervisor_handle() {
        use super::is_supervisor_pipe_fd;
        use std::os::unix::io::AsRawFd;

        let mut fds = [0; 2];
        // SAFETY: `pipe` writes two descriptors into a fully owned array.
        assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0, "pipe()");
        assert!(
            is_supervisor_pipe_fd(fds[0]),
            "a pipe is a supervisor handle"
        );
        // SAFETY: both descriptors came from the `pipe` call above and are closed once.
        unsafe {
            libc::close(fds[0]);
            libc::close(fds[1]);
        }

        let null = std::fs::File::open("/dev/null").expect("open /dev/null");
        assert!(
            !is_supervisor_pipe_fd(null.as_raw_fd()),
            "/dev/null is never a supervisor handle"
        );

        let file = tempfile::NamedTempFile::new().expect("temp file");
        assert!(
            !is_supervisor_pipe_fd(file.as_file().as_raw_fd()),
            "a regular file is never a supervisor handle"
        );

        assert!(
            !is_supervisor_pipe_fd(-1),
            "an unusable descriptor is never a supervisor handle"
        );
    }
}
