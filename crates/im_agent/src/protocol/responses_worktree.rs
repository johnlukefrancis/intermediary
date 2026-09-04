// Path: crates/im_agent/src/protocol/responses_worktree.rs
// Description: Agent-to-UI response payload naming the entries one worktree action produced

use serde::{Deserialize, Serialize};

use super::commands_worktree::WorktreeActionKind;

/// The answer to `worktreeAction`: which kind ran, and the repo-relative paths
/// it produced, in the order the request named them.
///
/// What "produced" means is the one thing the UI needs to select afterwards:
/// a delete reports the paths it removed, a move or a copy reports the
/// destination paths, and a rename reports the single new path. Nothing here
/// describes the rest of the worktree — the watcher and the next status read
/// own that, exactly as they do for an import.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeActionResult {
    pub repo_id: String,
    pub kind: WorktreeActionKind,
    pub entries: Vec<String>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::WorktreeActionResult;
    use crate::protocol::{UiResponse, WorktreeActionKind};

    #[test]
    fn the_result_round_trips_as_a_tagged_response() {
        let response = UiResponse::WorktreeActionResult(WorktreeActionResult {
            repo_id: "repo".to_string(),
            kind: WorktreeActionKind::Move,
            entries: vec!["app/src/a.txt".to_string()],
        });

        let wire = serde_json::to_value(&response).expect("serialize");
        assert_eq!(
            wire,
            json!({
                "type": "worktreeActionResult",
                "repoId": "repo",
                "kind": "move",
                "entries": ["app/src/a.txt"]
            })
        );

        let decoded: UiResponse = serde_json::from_value(wire).expect("deserialize");
        let UiResponse::WorktreeActionResult(result) = decoded else {
            panic!("expected a worktreeActionResult");
        };
        assert_eq!(result.kind, WorktreeActionKind::Move);
        assert_eq!(result.entries, vec!["app/src/a.txt".to_string()]);
    }
}
