// Path: src-tauri/src/lib/terminal/frames.rs
// Description: Wire shapes shared with the frontend terminal client: open request/result, exit frame, close reasons and outcomes

use crate::config::types::RepoRoot;
use serde::{Deserialize, Serialize};

/// Header carrying the session id on the raw-body `terminal_write` invoke.
pub const SESSION_HEADER: &str = "tauri-terminal-session";

/// Argument object of `terminal_open`; `session_id` is minted by the frontend
/// (a UUID string) so the JS write path is addressable before the open resolves.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalOpenRequest {
    pub session_id: String,
    pub repo_root: RepoRoot,
    pub cols: u16,
    pub rows: u16,
}

/// Result of `terminal_open`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalOpened {
    pub session_id: String,
    pub pid: u32,
    /// `CurrentBuildNumber` of the host, so xterm can enable ConPTY reflow; `None` off Windows
    pub windows_build_number: Option<u32>,
    /// Directory pwsh was started in
    pub start_dir: String,
    /// Command pwsh runs after its profile (the WSL-root entry), if any
    pub initial_command: Option<String>,
}

/// Why a session ended; recorded once per session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CloseReason {
    /// The shell exited on its own
    ChildExit,
    /// The user closed the tab
    Closed,
    /// The webview navigated or reloaded; its channels are gone
    WebviewNavigation,
    /// The app is exiting
    AppExit,
    /// The output pipe failed before the child exited
    ReaderError,
    /// The open sequence failed after the child had started
    OpenFailed,
}

/// JSON frame sent on the output channel after the last output byte.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalExitFrame {
    /// Always `"exit"`; the channel otherwise carries raw bytes
    pub kind: &'static str,
    pub session_id: String,
    pub code: Option<u32>,
    pub reason: CloseReason,
}

impl TerminalExitFrame {
    pub fn new(session_id: &str, code: Option<u32>, reason: CloseReason) -> Self {
        Self {
            kind: "exit",
            session_id: session_id.to_string(),
            code,
            reason,
        }
    }
}

/// What `terminal_close` (and the app-exit sweep) observed while ending a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "outcome")]
pub enum CloseOutcome {
    /// The console close ended the child within the budget
    Exited { code: Option<u32> },
    /// The Job Object had to terminate the tree
    Escalated { code: Option<u32> },
    /// The child exceeded the close ladder; ownership is retained until its final receipt
    StillAlive,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_frame_serializes_with_camel_case_reason() {
        let frame = TerminalExitFrame::new("abc", Some(0), CloseReason::ChildExit);
        let json = serde_json::to_string(&frame).expect("serialize");
        assert_eq!(
            json,
            r#"{"kind":"exit","sessionId":"abc","code":0,"reason":"childExit"}"#
        );
    }

    #[test]
    fn close_outcome_is_tagged() {
        let json =
            serde_json::to_string(&CloseOutcome::Escalated { code: None }).expect("serialize");
        assert_eq!(json, r#"{"outcome":"escalated","code":null}"#);
        let json = serde_json::to_string(&CloseOutcome::StillAlive).expect("serialize");
        assert_eq!(json, r#"{"outcome":"stillAlive"}"#);
    }
}
