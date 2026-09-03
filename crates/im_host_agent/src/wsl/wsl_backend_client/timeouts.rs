// Path: crates/im_host_agent/src/wsl/wsl_backend_client/timeouts.rs
// Description: The host->WSL request-timeout ladder and the agent-side worst case each tier covers

//! Every constant here bounds one whole forwarded request, and a forwarded
//! source-control request runs a *sequence* of Git commands in the WSL agent,
//! each with its own bound (read 20 s, index 60 s, commit 120 s, remote 180 s;
//! `crates/im_agent/src/source_control/runner.rs`). The numbers below are
//! therefore stated as sums of those bounds, not guesses.
//!
//! The unit that recurs is one status capture: `rev-parse --show-prefix`,
//! `status --porcelain=v2`, `diff --cached --quiet`, `rev-parse MERGE_HEAD`,
//! and `ls-files --stage` — five bounded reads, **100 s** worst case. Every
//! mutation reads status before it acts (section paths, discard stamps, the
//! commit precondition) and again after it, so a mutation's worst case is
//! `100 + <the mutating calls> + 100`.
//!
//! What a tier guarantees: it covers the request's whole end-to-end worst case
//! (every Git command it can run, including the follow-up status read) with a
//! 20 s margin, and the UI tier above it adds 30 s more. It is still not a claim
//! about physical finality: when it expires the host answers
//! `WSL_BACKEND_TIMEOUT`, the WSL action is `Passive` and keeps running, the
//! effect is `unknown`, and the UI reconciles against `mutationInProgress`
//! until the agent reports itself idle.

use std::time::Duration;

use im_agent::protocol::{SourceControlActionKind, UiCommand};

pub(super) const FORWARD_REQUEST_TIMEOUT_DEFAULT: Duration = Duration::from_secs(60);
pub(super) const FORWARD_REQUEST_TIMEOUT_CLIENT_HELLO: Duration = Duration::from_secs(12);
pub(super) const FORWARD_REQUEST_TIMEOUT_BUILD_BUNDLE: Duration = Duration::from_secs(5 * 60);

/// Reads. A diff is 2 bounded reads (prefix + diff) = 40 s, an image diff 3
/// (prefix + both sides) = 60 s. A status capture is 5 bounded reads = 100 s
/// worst case; 120 s covers it. UI tier above: 150 s.
pub(super) const FORWARD_REQUEST_TIMEOUT_SOURCE_CONTROL_READ: Duration = Duration::from_secs(120);

/// Stage / unstage. Section status 100 s + `add -A` / `reset -q` 60 s +
/// follow-up status 100 s = 260 s end to end; 280 s covers it. UI tier above: 310 s.
pub(super) const FORWARD_REQUEST_TIMEOUT_SOURCE_CONTROL_INDEX: Duration = Duration::from_secs(280);

/// Discard, its own class because it runs two index mutations: classification
/// status 100 s + `restore --worktree` 60 s + `reset -q` 60 s + follow-up
/// status 100 s = 320 s end to end; 340 s covers it. UI tier above: 370 s.
pub(super) const FORWARD_REQUEST_TIMEOUT_SOURCE_CONTROL_DISCARD: Duration =
    Duration::from_secs(340);

/// Commit. Precondition status 100 s + `rev-parse HEAD` 20 s + `commit` 120 s
/// + post-timeout HEAD re-check 20 s + follow-up status 100 s = 360 s end to
/// end; 380 s covers it. UI tier above: 410 s.
pub(super) const FORWARD_REQUEST_TIMEOUT_SOURCE_CONTROL_COMMIT: Duration = Duration::from_secs(380);

/// Push / pull. Push is the worst case: status 100 s + `remote` 20 s + `push`
/// 180 s + follow-up status 100 s = 400 s end to end; 420 s covers it. UI tier
/// above: 450 s.
pub(super) const FORWARD_REQUEST_TIMEOUT_SOURCE_CONTROL_REMOTE: Duration = Duration::from_secs(420);

/// Shutdown. The WSL agent drains for at most 450 s
/// (`im_agent::server::SHUTDOWN_EMERGENCY_BOUND`) and answers immediately
/// after; 470 s leaves room for the response hop while the host's own drain
/// loop (`shutdown_dispatch::drain_wsl_backend`) still governs the real
/// wall-clock bound with its own emergency deadline and retries.
pub(super) const FORWARD_REQUEST_TIMEOUT_SHUTDOWN: Duration = Duration::from_secs(470);

pub(super) fn timeout_for_command(command: &UiCommand) -> Duration {
    match command {
        UiCommand::ClientHello(_) => FORWARD_REQUEST_TIMEOUT_CLIENT_HELLO,
        UiCommand::BuildBundle(_) => FORWARD_REQUEST_TIMEOUT_BUILD_BUNDLE,
        UiCommand::Shutdown => FORWARD_REQUEST_TIMEOUT_SHUTDOWN,
        UiCommand::SourceControlStatus(_)
        | UiCommand::SourceControlDiff(_)
        | UiCommand::SourceControlImageDiff(_) => FORWARD_REQUEST_TIMEOUT_SOURCE_CONTROL_READ,
        UiCommand::SourceControlAction(command) => match command.action.kind() {
            SourceControlActionKind::Stage | SourceControlActionKind::Unstage => {
                FORWARD_REQUEST_TIMEOUT_SOURCE_CONTROL_INDEX
            }
            SourceControlActionKind::Discard => FORWARD_REQUEST_TIMEOUT_SOURCE_CONTROL_DISCARD,
            SourceControlActionKind::Commit => FORWARD_REQUEST_TIMEOUT_SOURCE_CONTROL_COMMIT,
            SourceControlActionKind::Push | SourceControlActionKind::Pull => {
                FORWARD_REQUEST_TIMEOUT_SOURCE_CONTROL_REMOTE
            }
        },
        UiCommand::SetOptions(_)
        | UiCommand::WatchRepo(_)
        | UiCommand::Refresh(_)
        | UiCommand::StageFile(_)
        | UiCommand::ReadTextFile(_)
        | UiCommand::ReadImageFile(_)
        | UiCommand::CancelBundleBuild(_)
        | UiCommand::GetRepoTopLevel(_)
        | UiCommand::ListRepoDirectory(_)
        | UiCommand::ListBundles(_)
        | UiCommand::GetTrFleetStatus(_)
        | UiCommand::TrFleetAction(_)
        | UiCommand::Unknown => FORWARD_REQUEST_TIMEOUT_DEFAULT,
    }
}
