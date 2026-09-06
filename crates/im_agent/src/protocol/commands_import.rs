// Path: crates/im_agent/src/protocol/commands_import.rs
// Description: UI-to-agent command payload for importing external OS files into a repo directory

use serde::{Deserialize, Serialize};

/// What an import does when a resolved destination already exists.
///
/// `Refuse` is the safe default the UI offers first: nothing is written and
/// the conflicting repo-relative paths come back in `details.conflicts` so the
/// user can decide. `Replace` carries exactly the destinations the user then
/// authorized — the paths that conflict list named, spelled the same way — and
/// authorizes nothing else. A destination that filled up while the dialog was
/// open was never shown to anyone, so it is refused with a fresh list rather
/// than overwritten.
///
/// There is deliberately no wire form meaning "replace whatever is in the
/// way": the bare string `"replace"` fails to deserialize, so the older
/// blanket spelling cannot be replayed against this agent as an authorization
/// nobody gave.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportConflictPolicy {
    Refuse,
    Replace(Vec<String>),
}

impl ImportConflictPolicy {
    /// Whether this repo-relative destination is one the user authorized
    /// replacing, and therefore the one question every write site asks per
    /// entry rather than once per action.
    ///
    /// The authorized paths are normalized and sorted once, before any
    /// planning, by `repos::worktree::normalize_authorization` — the only
    /// route by which a policy reaches a write — so this is a binary search.
    /// A scan would be quadratic in the entry ceiling, which bounds the
    /// authorization and the plan alike.
    pub fn replaces(&self, dest_rel: &str) -> bool {
        match self {
            Self::Refuse => false,
            Self::Replace(authorized) => authorized
                .binary_search_by(|path| path.as_str().cmp(dest_rel))
                .is_ok(),
        }
    }
}

/// One drop of external OS paths into one directory of a configured repo.
///
/// `directory` is repo-relative and slash-joined (`""` is the worktree root;
/// `"."` is tolerated and means the same). `sources` are absolute OS paths
/// exactly as the host delivered them — `C:\…`, `\\wsl$\<distro>\…`, or a
/// POSIX path — and the agent that owns the repo root translates them into
/// its own namespace before touching anything.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFilesCommand {
    pub repo_id: String,
    pub directory: String,
    pub sources: Vec<String>,
    pub on_conflict: ImportConflictPolicy,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::protocol::{
        EnvelopeKind, ImportConflictPolicy, InboundRequestEnvelope, RequestEnvelope, UiCommand,
    };

    #[test]
    fn import_files_round_trips_through_the_request_envelope() {
        let envelope = RequestEnvelope {
            kind: EnvelopeKind::Request,
            request_id: "req-1".to_string(),
            payload: UiCommand::ImportFiles(super::ImportFilesCommand {
                repo_id: "repo".to_string(),
                directory: "app/src".to_string(),
                sources: vec![
                    "C:\\Users\\dev\\notes.md".to_string(),
                    "\\\\wsl$\\Ubuntu\\home\\dev\\pics".to_string(),
                ],
                on_conflict: ImportConflictPolicy::Replace(vec!["app/src/notes.md".to_string()]),
            }),
        };

        let wire = serde_json::to_value(&envelope).expect("serialize");
        assert_eq!(
            wire,
            json!({
                "kind": "request",
                "requestId": "req-1",
                "payload": {
                    "type": "importFiles",
                    "repoId": "repo",
                    "directory": "app/src",
                    "sources": [
                        "C:\\Users\\dev\\notes.md",
                        "\\\\wsl$\\Ubuntu\\home\\dev\\pics"
                    ],
                    "onConflict": { "replace": ["app/src/notes.md"] }
                }
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
        let UiCommand::ImportFiles(command) = *payload else {
            panic!("expected an importFiles command");
        };
        assert_eq!(command.repo_id, "repo");
        assert_eq!(command.directory, "app/src");
        assert_eq!(command.sources.len(), 2);
        assert_eq!(
            command.on_conflict,
            ImportConflictPolicy::Replace(vec!["app/src/notes.md".to_string()])
        );
        assert_eq!(
            UiCommand::ImportFiles(command).command_type(),
            "importFiles"
        );
    }

    /// Both variants, both directions. `Refuse` is a bare word because it
    /// carries nothing; `Replace` is a tagged object because it carries the
    /// list that *is* the authorization.
    #[test]
    fn each_policy_variant_round_trips_through_its_own_wire_form() {
        for (policy, wire) in [
            (ImportConflictPolicy::Refuse, json!("refuse")),
            (
                ImportConflictPolicy::Replace(Vec::new()),
                json!({ "replace": [] }),
            ),
            (
                ImportConflictPolicy::Replace(vec![
                    "docs/a.md".to_string(),
                    "docs/b.md".to_string(),
                ]),
                json!({ "replace": ["docs/a.md", "docs/b.md"] }),
            ),
        ] {
            assert_eq!(serde_json::to_value(&policy).expect("serialize"), wire);
            let decoded: ImportConflictPolicy =
                serde_json::from_value(wire.clone()).expect("deserialize");
            assert_eq!(decoded, policy, "{wire}");
        }
    }

    /// The old wire form must not mean "replace everything". A client still
    /// sending the bare word is refused at the envelope, not quietly granted
    /// an authorization for every destination in the drop.
    #[test]
    fn the_bare_replace_string_is_not_an_authorization() {
        serde_json::from_value::<ImportConflictPolicy>(json!("replace"))
            .expect_err("a blanket replace has no wire form");
    }
}
