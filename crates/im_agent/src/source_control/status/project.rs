// Path: crates/im_agent/src/source_control/status/project.rs
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

use super::StatusCapture;

/// Whole-repository status projected onto the configured root: paths lose the
/// repo prefix, staged paths outside the root and paths not representable as
/// UTF-8 are counted instead of listed, and each list is sorted by path.
///
/// `index_ready` is Git's answer about the index alone; a repository with any
/// unmerged record is not committable however ready the index looks.
/// `snapshot_id` is left empty here and filled by `capture_status`, which is
/// the only step that knows the git dir the in-progress state lives in.
pub(super) fn project_status(
    prefix: &[u8],
    output: GitCommandOutput,
    index_ready: bool,
    index_tree_sha: String,
    mutation_in_progress: bool,
) -> Result<StatusCapture, AgentError> {
    let truncated = output.stdout_truncated;
    let parsed = parse_best_effort(output.stdout, truncated)?;
    let mut lists = Lists::default();
    for record in &parsed.records {
        lists.push(record, prefix);
    }
    lists.sort();
    let unmerged = lists.unmerged;
    let status = SourceControlStatus {
        detached: parsed.branch.is_none() && parsed.head_sha.is_some(),
        branch: parsed.branch,
        head_sha: parsed.head_sha,
        upstream: parsed.upstream,
        ahead: parsed.ahead,
        behind: parsed.behind,
        index: lists.index,
        worktree: lists.worktree,
        conflicts: lists.conflicts,
        committable: index_ready && !unmerged,
        index_tree_sha,
        // The snapshot identity spans this projection and the git dir's
        // in-progress state, which only the capture step has resolved; it fills
        // this in before the status leaves the agent.
        snapshot_id: String::new(),
        mutation_in_progress,
        omitted: lists.omitted,
        truncated,
        captured_at_iso: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
    };
    Ok(StatusCapture { status, unmerged })
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
    /// Any unmerged record in the whole repository, inside the root or not.
    unmerged: bool,
}

impl Lists {
    fn push(&mut self, record: &StatusRecord, prefix: &[u8]) {
        if record.is_unmerged() {
            self.unmerged = true;
        }
        let Some(relative) = strip_repo_prefix(record.current.as_bytes(), prefix) else {
            self.push_outside(record, prefix);
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
        let original_bytes = record.original.as_ref().map(|original| original.as_bytes());
        let original_inside = original_bytes
            .and_then(|bytes| strip_repo_prefix(bytes, prefix))
            .filter(|bytes| !bytes.is_empty());
        let original = original_inside.and_then(|bytes| String::from_utf8(bytes.to_vec()).ok());
        let index_letter = index_letter(record);
        let worktree_letter = record.xy.as_bytes().get(1).copied().unwrap_or(b'.');
        // A rename into the root from outside it: the entry belongs here, but
        // the deletion of the outside source travels with the same commit and
        // has to be counted. A copy leaves its source alone, so it counts
        // nothing.
        if index_letter == b'R' && original_bytes.is_some() && original_inside.is_none() {
            self.omitted.staged_outside_root += 1;
        }
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

    /// A record whose current path is outside the root. Staged content travels
    /// with a commit of the whole index and an unmerged path out there blocks
    /// that commit, so both are counted; worktree-only and untracked records
    /// are not this root's concern and are dropped silently. A staged rename
    /// out of the root still deletes its inside endpoint, and that deletion is
    /// listed so the user sees the inside file leave.
    fn push_outside(&mut self, record: &StatusRecord, prefix: &[u8]) {
        if record.is_unmerged() {
            self.omitted.unmerged_outside_root += 1;
            return;
        }
        if !is_staged(record) {
            return;
        }
        self.omitted.staged_outside_root += 1;
        if index_letter(record) != b'R' {
            return;
        }
        let Some(original) = record
            .original
            .as_ref()
            .and_then(|original| strip_repo_prefix(original.as_bytes(), prefix))
            .filter(|bytes| !bytes.is_empty())
        else {
            return;
        };
        match String::from_utf8(original.to_vec()) {
            Ok(path) => self.index.push(entry(
                path,
                None,
                SourceControlEntryArea::Index,
                SourceControlChange::Deleted,
            )),
            Err(_) => self.omitted.unrepresentable_path += 1,
        }
    }

    fn sort(&mut self) {
        for list in [&mut self.index, &mut self.worktree, &mut self.conflicts] {
            list.sort_by(|a, b| a.path.cmp(&b.path));
        }
    }
}

fn is_staged(record: &StatusRecord) -> bool {
    !record.is_untracked() && !record.is_unmerged() && index_letter(record) != b'.'
}

fn index_letter(record: &StatusRecord) -> u8 {
    record.xy.as_bytes().first().copied().unwrap_or(b'.')
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
        worktree_stamp: None,
        worktree_missing: false,
    }
}
