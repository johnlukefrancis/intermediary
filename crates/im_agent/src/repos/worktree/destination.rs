// Path: crates/im_agent/src/repos/worktree/destination.rs
// Description: Resolving the destination folder of a worktree write, bounding its replace authorization, and proving the paths it claims are distinct

use std::collections::{BTreeSet, HashSet};
use std::io;
use std::path::{Path, PathBuf};

use tokio::fs;

use crate::error::AgentError;
use crate::protocol::ImportConflictPolicy;
use crate::repos::MAX_IMPORT_ENTRIES;
use crate::source_control::normalize_path;

use super::entries::conflict_error;

/// The destination directory, resolved once. Canonical because the
/// self-write checks compare it against canonical source paths, and
/// containment against the canonical root is what makes a symlinked component
/// unable to carry the write outside the worktree.
///
/// `directory` is already normalized (`""` is the worktree root).
pub(crate) async fn resolve_destination(
    repo_root: &Path,
    directory: &str,
) -> Result<PathBuf, AgentError> {
    let canonical_root = fs::canonicalize(repo_root)
        .await
        .map_err(|error| AgentError::internal(format!("Failed to resolve repo root: {error}")))?;
    let target = if directory.is_empty() {
        canonical_root.clone()
    } else {
        repo_root.join(directory)
    };

    let canonical_target = fs::canonicalize(&target)
        .await
        .map_err(|error| match error.kind() {
            io::ErrorKind::NotFound => {
                AgentError::new("DIR_NOT_FOUND", "Directory does not exist")
            }
            _ => AgentError::internal(format!("Failed to resolve directory: {error}")),
        })?;
    if !canonical_target.starts_with(&canonical_root) {
        return Err(AgentError::new(
            "INVALID_PATH",
            "Path escapes configured repo root",
        ));
    }

    let metadata = fs::metadata(&canonical_target)
        .await
        .map_err(|error| AgentError::internal(format!("Failed to stat directory: {error}")))?;
    if !metadata.is_dir() {
        return Err(AgentError::new("NOT_DIRECTORY", "Path is not a directory"));
    }
    Ok(canonical_target)
}

/// The drop's replace authorization, proven and prepared before anything is
/// planned — the one place a policy is turned into something a write may ask.
///
/// Each authorized path is normalized exactly as the `ENTRY_CONFLICT`
/// `details.conflicts` list that produced it was, so the per-entry comparison
/// at the write is byte-exact and no spelling of a path can smuggle in an
/// authorization for another. A list longer than one drop may carry entries
/// answers no conflict list this agent could ever have reported, so it is
/// refused here rather than searched. Sorted and deduplicated because
/// `ImportConflictPolicy::replaces` binary-searches it once per written entry.
pub(crate) fn normalize_authorization(
    policy: &ImportConflictPolicy,
) -> Result<ImportConflictPolicy, AgentError> {
    let ImportConflictPolicy::Replace(authorized) = policy else {
        return Ok(ImportConflictPolicy::Refuse);
    };
    if authorized.len() > MAX_IMPORT_ENTRIES {
        return Err(AgentError::new(
            "INVALID_PATH",
            format!("An action may authorize at most {MAX_IMPORT_ENTRIES} replacements"),
        ));
    }
    let mut normalized = authorized
        .iter()
        .map(|path| normalize_path(path))
        .collect::<Result<Vec<String>, AgentError>>()?;
    normalized.sort();
    normalized.dedup();
    Ok(ImportConflictPolicy::Replace(normalized))
}

/// Two entries claiming one destination path. Whichever policy is in force,
/// one of them would silently lose, so the whole action is refused and names
/// the paths that collided.
///
/// The comparison is byte-exact on purpose. On a case-insensitive volume
/// `A.txt` and `a.txt` are one destination reached by two spellings, and this
/// function lets both through; that alias is caught at the write instead, by
/// the no-replace primitive every unauthorized destination is written with,
/// and reported as `ENTRY_CONFLICT`. Nothing is overwritten either way, and
/// nothing here probes the filesystem for its case rules to find out.
pub(crate) fn ensure_distinct_destinations<'a>(
    dest_rels: impl IntoIterator<Item = &'a str>,
) -> Result<(), AgentError> {
    let mut seen: HashSet<&str> = HashSet::new();
    let mut conflicts = BTreeSet::new();
    for dest_rel in dest_rels {
        if !seen.insert(dest_rel) {
            conflicts.insert(dest_rel.to_string());
        }
    }
    if conflicts.is_empty() {
        return Ok(());
    }
    Err(conflict_error(
        conflicts,
        "Two of the selected items would land on the same path",
    ))
}

/// Joins a repo-relative parent and one entry name into the slash form every
/// repo path uses on the wire.
pub(crate) fn join_relative(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_string()
    } else {
        format!("{parent}/{name}")
    }
}

#[cfg(test)]
mod tests {
    use super::{ensure_distinct_destinations, join_relative, normalize_authorization};
    use crate::protocol::ImportConflictPolicy;
    use crate::repos::MAX_IMPORT_ENTRIES;

    #[test]
    fn joins_onto_the_root_without_a_leading_separator() {
        assert_eq!(join_relative("", "a.txt"), "a.txt");
        assert_eq!(join_relative("app/src", "a.txt"), "app/src/a.txt");
    }

    #[test]
    fn one_destination_claimed_twice_is_a_conflict_naming_it() {
        ensure_distinct_destinations(["app/a.txt", "app/b.txt"]).expect("distinct");
        let error =
            ensure_distinct_destinations(["app/a.txt", "app/a.txt"]).expect_err("duplicate");
        assert_eq!(error.code(), "ENTRY_CONFLICT");
        assert_eq!(
            error.details().and_then(|details| details.get("conflicts")),
            Some(&serde_json::json!(["app/a.txt"]))
        );
    }

    /// An authorization is normalized to the spelling the conflict list used,
    /// sorted for the search every written entry makes, and asked with
    /// `replaces` rather than compared by hand at each site.
    #[test]
    fn an_authorization_is_normalized_sorted_and_asked_by_path() {
        let policy = normalize_authorization(&ImportConflictPolicy::Replace(vec![
            "./docs//b.md".to_string(),
            "docs/a.md".to_string(),
            "docs/a.md".to_string(),
        ]))
        .expect("authorization");

        assert_eq!(
            policy,
            ImportConflictPolicy::Replace(vec!["docs/a.md".to_string(), "docs/b.md".to_string()])
        );
        assert!(policy.replaces("docs/a.md"));
        assert!(policy.replaces("docs/b.md"));
        assert!(!policy.replaces("docs/c.md"));
        assert!(!ImportConflictPolicy::Refuse.replaces("docs/a.md"));
    }

    #[test]
    fn an_authorization_no_conflict_list_could_have_produced_is_refused() {
        let too_many = vec!["a.txt".to_string(); MAX_IMPORT_ENTRIES + 1];
        assert_eq!(
            normalize_authorization(&ImportConflictPolicy::Replace(too_many))
                .expect_err("too large")
                .code(),
            "INVALID_PATH"
        );
        assert_eq!(
            normalize_authorization(&ImportConflictPolicy::Replace(vec!["../x".to_string()]))
                .expect_err("traversal")
                .code(),
            "INVALID_PATH"
        );
    }
}
