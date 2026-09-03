// Path: src-tauri/src/lib/agent/supervisor/wsl_terminate_logging.rs
// Description: How a WSL emergency-stop outcome is named in the supervisor log

use crate::agent::wsl_agent_termination::WslTerminateOutcome;

/// The path a stop actually took, for the log. `kill` carries the count of
/// descendant groups and pids the in-distro sweep reached, because that number
/// is the only evidence the tree — not just the agent — was ended.
pub(super) fn outcome_label(outcome: WslTerminateOutcome) -> String {
    match outcome {
        WslTerminateOutcome::NoMatch => "no_match".to_string(),
        WslTerminateOutcome::TerminatedWithTerm => "drained_by_agent".to_string(),
        WslTerminateOutcome::TerminatedWithKill { signalled } => {
            format!("kill signalledTrees={signalled}")
        }
    }
}

/// Only the emergency escalation is a warning: it means an agent outlived its
/// own drain bound and this supervisor had to end its process trees.
pub(super) fn outcome_level(outcome: WslTerminateOutcome) -> &'static str {
    match outcome {
        WslTerminateOutcome::NoMatch | WslTerminateOutcome::TerminatedWithTerm => "info",
        WslTerminateOutcome::TerminatedWithKill { .. } => "warn",
    }
}
