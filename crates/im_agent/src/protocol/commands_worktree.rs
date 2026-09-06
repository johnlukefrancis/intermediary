// Path: crates/im_agent/src/protocol/commands_worktree.rs
// Description: UI-to-agent command payload for deleting, moving, copying, and renaming worktree entries

use serde::{Deserialize, Serialize};

use super::commands_import::ImportConflictPolicy;

/// What one `worktreeAction` request does to a configured repository's
/// worktree, tagged by `kind` the way every other action payload on this wire
/// is.
///
/// Every `paths`/`path` entry is a repo-relative slash path naming one thing
/// the user selected — never the worktree root itself — and `directory` is the
/// destination folder (`""` is the root, `"."` is tolerated and means the
/// same). A move and a copy answer to the same question an import answers when
/// a destination is taken, so they reuse its policy rather than minting a
/// second vocabulary for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum WorktreeAction {
    /// Removes the named entries. Recoverable by construction: each is claimed
    /// into this repository's discard quarantine instead of being unlinked, so
    /// the bytes stand until the next agent start.
    Delete { paths: Vec<String> },
    /// Moves the named entries into `directory`. A folder never merges into an
    /// existing folder of the same name — that would destroy what is already
    /// there — so `on_conflict` decides only the file-over-file case.
    Move {
        paths: Vec<String>,
        directory: String,
        on_conflict: ImportConflictPolicy,
    },
    /// Copies the named entries into `directory`, with exactly the import's
    /// semantics: a folder merges, files answer to `on_conflict`.
    Copy {
        paths: Vec<String>,
        directory: String,
        on_conflict: ImportConflictPolicy,
    },
    /// Renames one entry in place. `new_name` is a single name, never a path,
    /// and never replaces anything that already exists.
    Rename { path: String, new_name: String },
}

/// The action kind alone, for the answer and for the routing decisions that
/// must not carry a whole payload (the host's forwarding timeout tier).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WorktreeActionKind {
    Delete,
    Move,
    Copy,
    Rename,
}

impl WorktreeAction {
    pub fn kind(&self) -> WorktreeActionKind {
        match self {
            Self::Delete { .. } => WorktreeActionKind::Delete,
            Self::Move { .. } => WorktreeActionKind::Move,
            Self::Copy { .. } => WorktreeActionKind::Copy,
            Self::Rename { .. } => WorktreeActionKind::Rename,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeActionCommand {
    pub repo_id: String,
    pub action: WorktreeAction,
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::{WorktreeAction, WorktreeActionCommand, WorktreeActionKind};
    use crate::protocol::{
        EnvelopeKind, ImportConflictPolicy, InboundRequestEnvelope, RequestEnvelope, UiCommand,
    };

    /// One action out and back through the envelope the UI actually sends,
    /// asserting the whole wire shape rather than field names in isolation.
    fn round_trip(action: WorktreeAction, expected_payload: Value) -> WorktreeAction {
        let kind = action.kind();
        let envelope = RequestEnvelope {
            kind: EnvelopeKind::Request,
            request_id: "req-1".to_string(),
            payload: UiCommand::WorktreeAction(WorktreeActionCommand {
                repo_id: "repo".to_string(),
                action,
            }),
        };

        let wire = serde_json::to_value(&envelope).expect("serialize");
        assert_eq!(
            wire,
            json!({
                "kind": "request",
                "requestId": "req-1",
                "payload": expected_payload
            })
        );

        let decoded: InboundRequestEnvelope =
            serde_json::from_value(wire).expect("deserialize inbound");
        let InboundRequestEnvelope::Request {
            request_id,
            payload,
        } = decoded
        else {
            panic!("expected a request envelope");
        };
        assert_eq!(request_id, "req-1");
        let UiCommand::WorktreeAction(command) = *payload else {
            panic!("expected a worktreeAction command");
        };
        assert_eq!(command.repo_id, "repo");
        assert_eq!(command.action.kind(), kind);
        assert_eq!(
            UiCommand::WorktreeAction(WorktreeActionCommand {
                repo_id: "repo".to_string(),
                action: command.action.clone(),
            })
            .command_type(),
            "worktreeAction"
        );
        command.action
    }

    #[test]
    fn delete_round_trips_through_the_request_envelope() {
        let action = round_trip(
            WorktreeAction::Delete {
                paths: vec!["app/a.txt".to_string(), "app/old".to_string()],
            },
            json!({
                "type": "worktreeAction",
                "repoId": "repo",
                "action": { "kind": "delete", "paths": ["app/a.txt", "app/old"] }
            }),
        );
        assert_eq!(action.kind(), WorktreeActionKind::Delete);
    }

    #[test]
    fn move_round_trips_through_the_request_envelope() {
        let action = round_trip(
            WorktreeAction::Move {
                paths: vec!["app/a.txt".to_string()],
                directory: "app/src".to_string(),
                on_conflict: ImportConflictPolicy::Refuse,
            },
            json!({
                "type": "worktreeAction",
                "repoId": "repo",
                "action": {
                    "kind": "move",
                    "paths": ["app/a.txt"],
                    "directory": "app/src",
                    "onConflict": "refuse"
                }
            }),
        );
        assert_eq!(action.kind(), WorktreeActionKind::Move);
    }

    #[test]
    fn copy_round_trips_through_the_request_envelope() {
        let action = round_trip(
            WorktreeAction::Copy {
                paths: vec!["app/a.txt".to_string()],
                directory: String::new(),
                on_conflict: ImportConflictPolicy::Replace(vec!["a.txt".to_string()]),
            },
            json!({
                "type": "worktreeAction",
                "repoId": "repo",
                "action": {
                    "kind": "copy",
                    "paths": ["app/a.txt"],
                    "directory": "",
                    "onConflict": { "replace": ["a.txt"] }
                }
            }),
        );
        assert_eq!(action.kind(), WorktreeActionKind::Copy);
    }

    #[test]
    fn rename_round_trips_through_the_request_envelope() {
        let action = round_trip(
            WorktreeAction::Rename {
                path: "app/a.txt".to_string(),
                new_name: "b.txt".to_string(),
            },
            json!({
                "type": "worktreeAction",
                "repoId": "repo",
                "action": { "kind": "rename", "path": "app/a.txt", "newName": "b.txt" }
            }),
        );
        assert_eq!(action.kind(), WorktreeActionKind::Rename);
    }
}
