// Path: crates/im_agent/src/source_control/status_project.rs
// Description: Projects parsed porcelain-v2 status onto the SourceControlStatus wire shape for one root

use chrono::{SecondsFormat, Utc};
use im_bundle::git::{
    parse_porcelain, strip_repo_prefix, GitCommandOutput, PorcelainStatus, StatusRecord,
};

use crate::error::AgentError;
use crate::protocol::{
    SourceControlChange, SourceControlEntry, SourceControlEntryArea, SourceControlOmitted,
    SourceControlStatus,
};

/// Whole-repository status projected onto the configured root: paths lose the
/// repo prefix, staged paths outside the root and paths not representable as
/// UTF-8 are counted instead of listed, and each list is sorted by path.
pub(super) fn project_status(
    prefix: &[u8],
    output: GitCommandOutput,
    committable: bool,
) -> Result<SourceControlStatus, AgentError> {
    let truncated = output.stdout_truncated;
    let parsed = parse_best_effort(output.stdout, truncated)?;
    let mut lists = Lists::default();
    for record in &parsed.records {
        lists.push(record, prefix);
    }
    lists.sort();
    Ok(SourceControlStatus {
        detached: parsed.branch.is_none() && parsed.head_sha.is_some(),
        branch: parsed.branch,
        head_sha: parsed.head_sha,
        upstream: parsed.upstream,
        ahead: parsed.ahead,
        behind: parsed.behind,
        index: lists.index,
        worktree: lists.worktree,
        conflicts: lists.conflicts,
        committable,
        omitted: lists.omitted,
        truncated,
        captured_at_iso: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
    })
}

/// A truncated capture is cut back to its last NUL so only complete fields
/// reach the strict parser; the torn trailing record is dropped.
fn parse_best_effort(mut stdout: Vec<u8>, truncated: bool) -> Result<PorcelainStatus, AgentError> {
    if truncated {
        match stdout.iter().rposition(|byte| *byte == 0) {
            Some(last_nul) => stdout.truncate(last_nul + 1),
            None => stdout.clear(),
        }
    }
    parse_porcelain(&stdout).map_err(|error| {
        AgentError::new(
            "GIT_COMMAND_FAILED",
            format!("Git status output could not be parsed: {error}"),
        )
    })
}

#[derive(Default)]
struct Lists {
    index: Vec<SourceControlEntry>,
    worktree: Vec<SourceControlEntry>,
    conflicts: Vec<SourceControlEntry>,
    omitted: SourceControlOmitted,
}

impl Lists {
    fn push(&mut self, record: &StatusRecord, prefix: &[u8]) {
        let Some(relative) = strip_repo_prefix(record.current.as_bytes(), prefix) else {
            // Only staged content outside the root travels with a commit of
            // the whole index; worktree-only, untracked, and unmerged records
            // out there are not this root's concern and are dropped silently.
            if is_staged(record) {
                self.omitted.staged_outside_root += 1;
            }
            return;
        };
        let Ok(path) = String::from_utf8(relative.to_vec()) else {
            self.omitted.unrepresentable_path += 1;
            return;
        };
        if record.is_untracked() {
            self.worktree.push(entry(
                path,
                None,
                SourceControlEntryArea::Worktree,
                SourceControlChange::Untracked,
            ));
            return;
        }
        if record.is_unmerged() {
            self.conflicts.push(entry(
                path,
                None,
                SourceControlEntryArea::Conflict,
                SourceControlChange::Unmerged,
            ));
            return;
        }
        // A rename source that cannot be stripped or represented drops only
        // `original_path`; the entry itself is still listed.
        let original = record
            .original
            .as_ref()
            .and_then(|original| strip_repo_prefix(original.as_bytes(), prefix))
            .filter(|bytes| !bytes.is_empty())
            .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok());
        let xy = record.xy.as_bytes();
        let index_letter = xy.first().copied().unwrap_or(b'.');
        let worktree_letter = xy.get(1).copied().unwrap_or(b'.');
        if index_letter != b'.' {
            let change = change_from(index_letter);
            self.index.push(entry(
                path.clone(),
                rename_source(change, original.clone()),
                SourceControlEntryArea::Index,
                change,
            ));
        }
        if worktree_letter != b'.' {
            let change = change_from(worktree_letter);
            self.worktree.push(entry(
                path,
                rename_source(change, original),
                SourceControlEntryArea::Worktree,
                change,
            ));
        }
    }

    fn sort(&mut self) {
        for list in [&mut self.index, &mut self.worktree, &mut self.conflicts] {
            list.sort_by(|a, b| a.path.cmp(&b.path));
        }
    }
}

fn is_staged(record: &StatusRecord) -> bool {
    let index_letter = record.xy.as_bytes().first().copied().unwrap_or(b'.');
    !record.is_untracked() && !record.is_unmerged() && index_letter != b'.'
}

/// Porcelain-v2 change letter for either column. Any letter Git may add
/// later still surfaces as a visible modification rather than vanishing.
fn change_from(letter: u8) -> SourceControlChange {
    match letter {
        b'A' => SourceControlChange::Added,
        b'D' => SourceControlChange::Deleted,
        b'R' => SourceControlChange::Renamed,
        b'C' => SourceControlChange::Copied,
        b'T' => SourceControlChange::TypeChanged,
        _ => SourceControlChange::Modified,
    }
}

fn rename_source(change: SourceControlChange, original: Option<String>) -> Option<String> {
    match change {
        SourceControlChange::Renamed | SourceControlChange::Copied => original,
        _ => None,
    }
}

fn entry(
    path: String,
    original_path: Option<String>,
    area: SourceControlEntryArea,
    change: SourceControlChange,
) -> SourceControlEntry {
    SourceControlEntry {
        path,
        original_path,
        area,
        change,
    }
}
