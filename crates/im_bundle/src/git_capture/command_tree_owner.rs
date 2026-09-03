// Path: crates/im_bundle/src/git_capture/command_tree_owner.rs
// Description: Getting a process-tree owner for one Git command, and how being refused one is reported

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};

use super::command::KillPolicy;
use super::command_failure::{GitCommandFailure, GitCommandFailureKind};
use super::command_tree::{GitProcessTree, TreeOwner};

static UNOWNED_READ_LOGGED: AtomicBool = AtomicBool::new(false);

/// The tree this command will run inside. A mutation that cannot be given one
/// is refused here, before anything is spawned; a read is told once per process
/// that it is running unowned and continues.
pub(super) fn owner_for(kill_policy: KillPolicy) -> Result<GitProcessTree, GitCommandFailure> {
    match TreeOwner::new() {
        Ok(owner) => Ok(GitProcessTree::from_owner(owner)),
        Err(error) if kill_policy == KillPolicy::Graceful => Err(no_owner_failure(&error, false)),
        Err(error) => {
            log_unowned_read_once(&error);
            Ok(GitProcessTree::from_owner(TreeOwner::unowned()))
        }
    }
}

/// `spawned` records whether a child already existed when ownership failed:
/// a pre-spawn refusal proves Git never ran, an attach failure only proves it
/// was killed in the same breath.
pub(super) fn no_owner_failure(error: &io::Error, spawned: bool) -> GitCommandFailure {
    GitCommandFailure {
        kind: GitCommandFailureKind::NoProcessTreeOwner { spawned },
        exit_code: None,
        stdout: Vec::new(),
        stderr: format!("Git process tree owner unavailable: {error}").into_bytes(),
    }
}

/// One line per process. A read that runs unowned is worth knowing, but the
/// status poll behind the source-control panel would turn a per-command line
/// into a log flood.
pub(super) fn log_unowned_read_once(error: &io::Error) {
    if UNOWNED_READ_LOGGED.swap(true, Ordering::Relaxed) {
        return;
    }
    eprintln!("im_bundle: git reads are running without a process tree owner: {error}");
}

#[cfg(test)]
mod tests {
    use super::{no_owner_failure, GitCommandFailureKind};
    use std::io;

    /// A refusal that does not say what the OS said leaves the operator with
    /// "Git did not run" and nothing to act on.
    #[test]
    fn the_refusal_names_the_os_error() {
        let error = io::Error::from_raw_os_error(5);
        let failure = no_owner_failure(&error, false);
        assert_eq!(
            failure.kind,
            GitCommandFailureKind::NoProcessTreeOwner { spawned: false }
        );
        assert!(failure.message().contains(&error.to_string()));
    }
}
