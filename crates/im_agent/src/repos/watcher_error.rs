// Path: crates/im_agent/src/repos/watcher_error.rs
// Description: Watcher error classification and event shaping

use crate::protocol::{AgentErrorCode, AgentErrorDetails, AgentErrorEvent};

const DOC_PATH: &str = "docs/commands/fix_inotify_limits.md";
const MOUNTED_WINDOWS_DOC_PATH: &str = "docs/usage/agent_wsl_bruised_states.md";

pub fn build_watcher_error_event(
    repo_id: &str,
    raw_message: String,
    raw_code: Option<String>,
) -> AgentErrorEvent {
    let info = match raw_code.as_deref() {
        Some("ENOSPC") => WatcherErrorInfo {
            code: Some(AgentErrorCode::WatcherInotifyLimit),
            message: "Repo watcher hit the inotify watch limit (ENOSPC).".to_string(),
            raw_code,
            raw_message,
            doc_path: Some(DOC_PATH.to_string()),
        },
        Some("EMFILE") => WatcherErrorInfo {
            code: Some(AgentErrorCode::WatcherFdLimit),
            message: "Repo watcher hit the open file descriptor limit (EMFILE).".to_string(),
            raw_code,
            raw_message,
            doc_path: Some(DOC_PATH.to_string()),
        },
        _ => WatcherErrorInfo {
            code: None,
            message: "Repo watcher encountered an error.".to_string(),
            raw_code,
            raw_message,
            doc_path: None,
        },
    };

    AgentErrorEvent::new(
        "watcher",
        format!("{} Repo: {repo_id}", info.message),
        Some(AgentErrorDetails {
            code: info.code,
            doc_path: info.doc_path,
            repo_id: Some(repo_id.to_string()),
            raw_code: info.raw_code,
            raw_message: Some(info.raw_message),
        }),
    )
}

pub fn build_mounted_windows_path_warning_event(repo_id: &str, repo_root: &str) -> AgentErrorEvent {
    AgentErrorEvent::new(
        "watcher",
        format!(
            "Repo watcher is running on a mounted Windows path and may miss changes. Repo: {repo_id}"
        ),
        Some(AgentErrorDetails {
            code: Some(AgentErrorCode::WatcherMountedWindowsPathRisk),
            doc_path: Some(MOUNTED_WINDOWS_DOC_PATH.to_string()),
            repo_id: Some(repo_id.to_string()),
            raw_code: Some("MOUNTED_WINDOWS_PATH_RISK".to_string()),
            raw_message: Some(format!("Mounted Windows path: {repo_root}")),
        }),
    )
}

struct WatcherErrorInfo {
    code: Option<AgentErrorCode>,
    message: String,
    raw_code: Option<String>,
    raw_message: String,
    doc_path: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::build_mounted_windows_path_warning_event;
    use crate::protocol::AgentErrorCode;

    #[test]
    fn mounted_windows_path_warning_includes_code_and_doc_link() {
        let event = build_mounted_windows_path_warning_event("screenshots", "/mnt/c/Users/john/Pictures");
        assert_eq!(event.scope, "watcher");
        let details = event.details.expect("details");
        assert_eq!(
            details.code,
            Some(AgentErrorCode::WatcherMountedWindowsPathRisk)
        );
        assert_eq!(
            details.doc_path.as_deref(),
            Some("docs/usage/agent_wsl_bruised_states.md")
        );
        assert_eq!(details.repo_id.as_deref(), Some("screenshots"));
        assert_eq!(
            details.raw_code.as_deref(),
            Some("MOUNTED_WINDOWS_PATH_RISK")
        );
    }
}
