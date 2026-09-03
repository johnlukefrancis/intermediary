// Path: crates/im_bundle/src/git_capture/fake_git.rs
// Description: Test-only fake Git scripts handed to a test only once the kernel will exec them

use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

/// First argument of the readiness probe. The runner always passes
/// `--no-pager` first, so a real run never takes the probe's early exit.
const PROBE_ARG: &str = "--im-fake-git-probe";
const PROBE_ATTEMPTS: u32 = 20;
const PROBE_BACKOFF: Duration = Duration::from_millis(25);

/// Writes `body` (shell lines, no shebang) as an executable fake Git at
/// `dir/name` and returns its path only once an `execve` of it has succeeded.
///
/// A freshly written file is `ETXTBSY` for as long as any process holds a
/// write descriptor on it. `fs::write` closes ours before the file is made
/// executable, but a sibling test forking its own child inside that window
/// inherits the descriptor and keeps the inode busy until that child execs —
/// which is enough to make the spawn under test fail instead of running.
/// The probe below is what closes the race: it execs the script itself,
/// harmlessly because the preamble exits on `PROBE_ARG`, and retries while
/// the kernel reports `ETXTBSY`. Nothing opens the file for writing again, so
/// one successful exec makes the returned path permanently safe to spawn.
pub(super) fn write_fake_git(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, script(body)).expect("write fake git");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
        .expect("fake git permissions");
    await_executable(&path);
    path
}

fn script(body: &str) -> String {
    format!("#!/bin/sh\nif [ \"$1\" = \"{PROBE_ARG}\" ]; then exit 0; fi\n{body}")
}

fn await_executable(path: &Path) {
    let mut busy: Option<io::Error> = None;
    for _ in 0..PROBE_ATTEMPTS {
        match probe(path) {
            Ok(status) => {
                assert!(
                    status.success(),
                    "fake git {} failed its probe: {status}",
                    path.display()
                );
                return;
            }
            Err(error) if error.raw_os_error() == Some(libc::ETXTBSY) => {
                busy = Some(error);
                std::thread::sleep(PROBE_BACKOFF);
            }
            Err(error) => panic!("fake git {} did not exec: {error}", path.display()),
        }
    }
    panic!(
        "fake git {} stayed text-busy for {PROBE_ATTEMPTS} probes: {busy:?}",
        path.display()
    );
}

fn probe(path: &Path) -> io::Result<std::process::ExitStatus> {
    Command::new(path)
        .arg(PROBE_ARG)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
}
