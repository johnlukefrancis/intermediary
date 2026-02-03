// Path: crates/im_agent/src/repos/watcher_error.rs
// Description: Watcher error classification and event shaping

use crate::protocol::{AgentErrorCode, AgentErrorDetails, AgentErrorEvent};

const DOC_PATH: &str = "docs/commands/fix_inotify_limits.md";

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

struct WatcherErrorInfo {
    code: Option<AgentErrorCode>,
    message: String,
    raw_code: Option<String>,
    raw_message: String,
    doc_path: Option<String>,
}
