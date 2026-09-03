// Path: src-tauri/src/lib/agent/supervisor/state.rs
// Description: Shared supervisor process state and process-kind labels

use super::graceful_stop::GracefulStopPath;
use crate::agent::wsl_process_control::WslLaunchTarget;
use im_bundle::process_job::JobHandle;
use std::process::Child;
use std::time::Instant;

#[derive(Debug, Clone, Copy)]
pub(super) enum ProcessKind {
    Host,
    Wsl,
}

impl ProcessKind {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Host => "Host agent",
            Self::Wsl => "WSL agent",
        }
    }

    pub(super) fn log_key(self) -> &'static str {
        match self {
            Self::Host => "host",
            Self::Wsl => "wsl",
        }
    }
}

/// A supervisor-owned process and the owner of the tree it can start. The two
/// travel as one value so a stop can never reach the child while the job that
/// holds its descendants is left behind, or the reverse.
///
/// `job` is `None` for a process whose tree this supervisor never owned: an
/// agent that was already listening when we started and was adopted by port and
/// token alone (`host::should_adopt_running_host`), and the WSL launcher, whose
/// real work runs inside the distro where a Windows job object reaches nothing.
#[derive(Debug)]
pub(super) struct SupervisedChild {
    pub child: Child,
    pub job: Option<JobHandle>,
}

impl SupervisedChild {
    pub(super) fn owned(child: Child, job: JobHandle) -> Self {
        Self {
            child,
            job: Some(job),
        }
    }
}

/// A child we spawned without owning its tree.
impl From<Child> for SupervisedChild {
    fn from(child: Child) -> Self {
        Self { child, job: None }
    }
}

#[derive(Debug, Default)]
pub(super) struct ManagedProcessState {
    pub process: Option<SupervisedChild>,
    pub last_spawn_at: Option<Instant>,
}

/// Durable identity of the WSL backend we manage this session (distro + reserved port). Unlike
/// `wsl_launch_target` — which is cleared and re-derived on every ensure pass — this survives so
/// config-less callers (`stop`, app exit) can reclaim the backend by port even when they hold no
/// launch target (adopted/reconnected backend, or a health-check race).
#[derive(Debug, Clone)]
pub(super) struct WslBackendHandle {
    pub distro: Option<String>,
    pub port: u16,
}

/// Durable identity of the host agent this session owns: the port it serves and
/// the token that authenticates us to it. `stop`, `restart`, and app exit carry
/// no config, and a graceful shutdown has to reach the same socket the
/// supervisor started or adopted.
#[derive(Debug, Clone)]
pub(super) struct HostBackendHandle {
    pub port: u16,
    pub ws_token: String,
}

#[derive(Debug, Default)]
pub(super) struct AgentSupervisorState {
    pub host: ManagedProcessState,
    pub wsl: ManagedProcessState,
    pub wsl_launch_target: Option<WslLaunchTarget>,
    pub last_host_backend: Option<HostBackendHandle>,
    pub last_wsl_backend: Option<WslBackendHandle>,
    pub last_error: Option<String>,
    /// How the host agent's most recent stop actually ended
    /// (`graceful_stop::stop_host_gracefully`). App-exit teardown reads this
    /// to decide whether the WSL distro is safe to terminate: never while
    /// finality came back `Unknown`.
    pub last_host_stop_finality: Option<GracefulStopPath>,
}

/// A child that stays alive until something stops it, for the tests that need a
/// real process to own, record, and kill. One owner for the whole supervisor's
/// tests, so no test module keeps a second spawn recipe.
#[cfg(test)]
pub(super) fn spawn_test_sleeper() -> Child {
    use std::process::{Command, Stdio};

    #[cfg(windows)]
    let mut command = {
        let mut command = Command::new("cmd");
        command.args(["/C", "ping", "-n", "31", "127.0.0.1"]);
        command
    };
    #[cfg(not(windows))]
    let mut command = {
        let mut command = Command::new("sh");
        command.args(["-c", "sleep 30"]);
        command
    };
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn the test sleeper")
}

/// A child that has already finished by the time the test looks at it.
#[cfg(test)]
pub(super) fn spawn_test_exited_child() -> Child {
    use std::process::{Command, Stdio};

    #[cfg(windows)]
    let mut command = {
        let mut command = Command::new("cmd");
        command.args(["/C", "exit", "0"]);
        command
    };
    #[cfg(not(windows))]
    let mut command = {
        let mut command = Command::new("sh");
        command.args(["-c", "exit 0"]);
        command
    };
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn the test child");
    child.wait().expect("wait for the test child");
    child
}

pub(super) fn process_state(
    state: &AgentSupervisorState,
    kind: ProcessKind,
) -> &ManagedProcessState {
    match kind {
        ProcessKind::Host => &state.host,
        ProcessKind::Wsl => &state.wsl,
    }
}

pub(super) fn process_state_mut(
    state: &mut AgentSupervisorState,
    kind: ProcessKind,
) -> &mut ManagedProcessState {
    match kind {
        ProcessKind::Host => &mut state.host,
        ProcessKind::Wsl => &mut state.wsl,
    }
}
