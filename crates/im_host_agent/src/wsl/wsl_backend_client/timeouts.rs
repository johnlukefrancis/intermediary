// Path: crates/im_host_agent/src/wsl/wsl_backend_client/timeouts.rs
// Description: The host->WSL request-timeout ladder and the agent-side worst case each tier covers

//! Forwarding budgets cover the full sequence of bounded agent operations.
//! Mutation timeouts preserve unknown outcomes for reconciliation; UI budgets add 30 s.

use std::time::Duration;

use im_agent::protocol::{SourceControlActionKind, UiCommand, WorktreeActionKind};

pub(super) const FORWARD_REQUEST_TIMEOUT_DEFAULT: Duration = Duration::from_secs(60);
pub(super) const FORWARD_REQUEST_TIMEOUT_CLIENT_HELLO: Duration = Duration::from_secs(12);
pub(super) const FORWARD_REQUEST_TIMEOUT_BUILD_BUNDLE: Duration = Duration::from_secs(5 * 60);

/// Status needs five reads (100 s); 120 s includes forwarding margin.
pub(super) const FORWARD_REQUEST_TIMEOUT_SOURCE_CONTROL_READ: Duration = Duration::from_secs(120);

/// Stage / unstage. Section status 100 s + `add -A` / `reset -q` 60 s +
/// follow-up status 100 s = 260 s end to end; 280 s covers it. UI tier above: 310 s.
pub(super) const FORWARD_REQUEST_TIMEOUT_SOURCE_CONTROL_INDEX: Duration = Duration::from_secs(280);

/// Discard: two status captures + restore + reset = 320 s; forwarding adds 20 s.
pub(super) const FORWARD_REQUEST_TIMEOUT_SOURCE_CONTROL_DISCARD: Duration =
    Duration::from_secs(340);

/// Commit: pre/post status + HEAD read/recheck + commit = 360 s; forwarding adds 20 s.
pub(super) const FORWARD_REQUEST_TIMEOUT_SOURCE_CONTROL_COMMIT: Duration = Duration::from_secs(380);

/// Remote: two status captures + remote read + push = 400 s; forwarding adds 20 s.
pub(super) const FORWARD_REQUEST_TIMEOUT_SOURCE_CONTROL_REMOTE: Duration = Duration::from_secs(420);

/// The agent drains for 450 s; 470 s leaves a response hop inside the host drain owner.
pub(super) const FORWARD_REQUEST_TIMEOUT_SHUTDOWN: Duration = Duration::from_secs(470);

/// Import: 20 s lock resolution + 280 s copy budget; expiry is an unknown outcome.
pub(super) const FORWARD_REQUEST_TIMEOUT_IMPORT_FILES: Duration = Duration::from_secs(300);

/// Entry mutations budget lock/location resolution; recursive copy uses the import tier.
pub(super) const FORWARD_REQUEST_TIMEOUT_WORKTREE_ENTRY: Duration = Duration::from_secs(60);

pub(super) fn timeout_for_command(command: &UiCommand) -> Duration {
    match command {
        UiCommand::ClientHello(_) => FORWARD_REQUEST_TIMEOUT_CLIENT_HELLO,
        UiCommand::BuildBundle(_) => FORWARD_REQUEST_TIMEOUT_BUILD_BUNDLE,
        UiCommand::Shutdown => FORWARD_REQUEST_TIMEOUT_SHUTDOWN,
        UiCommand::ImportFiles(_) => FORWARD_REQUEST_TIMEOUT_IMPORT_FILES,
        UiCommand::WorktreeAction(command) => match command.action.kind() {
            WorktreeActionKind::Copy => FORWARD_REQUEST_TIMEOUT_IMPORT_FILES,
            WorktreeActionKind::Delete | WorktreeActionKind::Move | WorktreeActionKind::Rename => {
                FORWARD_REQUEST_TIMEOUT_WORKTREE_ENTRY
            }
        },
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
        | UiCommand::InspectBundleEntry(_)
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
