// Path: crates/im_agent/src/repos/delta/unified_patch.rs
// Description: Hunks-only unified patch text and exact stats for one delta

use std::time::Instant;

use similar::{DiffOp, TextDiff};

use crate::protocol::DeltaStats;

use super::{CONTEXT_RADIUS, PATCH_MAX_BYTES};

/// A patch in the grammar the viewer's `parsePatch` expects: hunk headers plus
/// ` `, `+` and `-` rows, with no `diff --git` or `---`/`+++` file headers, so
/// the very first line is always a hunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PatchOutput {
    pub(crate) patch: String,
    /// Always measured over the whole diff, even when `patch` was cut.
    pub(crate) stats: DeltaStats,
    pub(crate) truncated: bool,
}

/// Diffs two texts under a wall-clock deadline. Past the deadline `similar`
/// approximates rather than running long on the blocking pool.
pub(crate) fn compute_patch(old: &str, new: &str, deadline: Instant) -> PatchOutput {
    let diff = TextDiff::configure()
        .deadline(deadline)
        .diff_lines(old, new);
    let (added, removed) = count_changed_lines(diff.ops());

    let mut unified = diff.unified_diff();
    unified
        .context_radius(CONTEXT_RADIUS)
        .missing_newline_hint(false);

    let mut patch = String::new();
    let mut hunks: u32 = 0;
    let mut truncated = false;
    for hunk in unified.iter_hunks() {
        hunks = hunks.saturating_add(1);
        if truncated {
            continue;
        }
        let rendered = hunk.to_string();
        if patch.len().saturating_add(rendered.len()) <= PATCH_MAX_BYTES {
            patch.push_str(&rendered);
            continue;
        }
        truncated = true;
        if patch.is_empty() {
            // One hunk wider than the whole budget (a full-file rewrite): keep
            // what fits, cut on a line boundary, so the card never rests empty.
            patch.push_str(line_prefix(&rendered, PATCH_MAX_BYTES));
        }
    }

    PatchOutput {
        patch,
        stats: DeltaStats {
            added,
            removed,
            hunks,
            new_lines: count_lines(new),
        },
        truncated,
    }
}

/// The deletion card: the last known content as one all-removed hunk.
pub(crate) fn all_removed_patch(text: &str) -> PatchOutput {
    let lines = count_lines(text);
    let (patch, truncated) =
        single_hunk(|rows| format!("@@ -1,{rows} +0,0 @@\n"), text, '-', lines);
    PatchOutput {
        patch,
        stats: DeltaStats {
            added: 0,
            removed: lines,
            hunks: u32::from(lines > 0),
            new_lines: 0,
        },
        truncated,
    }
}

/// The new-file card: every line added, no baseline.
pub(crate) fn all_added_patch(text: &str) -> PatchOutput {
    let lines = count_lines(text);
    let (patch, truncated) =
        single_hunk(|rows| format!("@@ -0,0 +1,{rows} @@\n"), text, '+', lines);
    PatchOutput {
        patch,
        stats: DeltaStats {
            added: lines,
            removed: 0,
            hunks: u32::from(lines > 0),
            new_lines: lines,
        },
        truncated,
    }
}

/// Renders every line under one prefix, stopping on the last whole line that
/// keeps the hunk (header included) inside `PATCH_MAX_BYTES`. The header counts
/// the rows actually emitted, never the rows that were cut, so a truncated card
/// still parses as a hunk whose header matches its body; `stats` keeps the full
/// totals. Budgeting against the full-count header is safe because a smaller
/// row count can never render a longer header.
fn single_hunk(
    header_for: impl Fn(u32) -> String,
    text: &str,
    prefix: char,
    lines: u32,
) -> (String, bool) {
    if lines == 0 {
        return (String::new(), false);
    }
    let budget = PATCH_MAX_BYTES.saturating_sub(header_for(lines).len());
    let mut body = String::new();
    let mut rows: u32 = 0;
    let mut truncated = false;
    for line in text.lines() {
        let row = line.len().saturating_add(2);
        if body.len().saturating_add(row) > budget {
            truncated = true;
            break;
        }
        body.push(prefix);
        body.push_str(line);
        body.push('\n');
        rows = rows.saturating_add(1);
    }
    if rows == 0 {
        // One line wider than the whole budget: a bare header would be a hunk
        // claiming rows it does not carry, so the card rests on the stats alone.
        return (String::new(), true);
    }
    let mut patch = header_for(rows);
    patch.push_str(&body);
    (patch, truncated)
}

/// The longest whole-line prefix of `rendered` that fits `limit` bytes.
fn line_prefix(rendered: &str, limit: usize) -> &str {
    let mut end = 0;
    for (index, byte) in rendered.bytes().enumerate() {
        if byte != b'\n' {
            continue;
        }
        if index + 1 > limit {
            break;
        }
        end = index + 1;
    }
    rendered.get(..end).unwrap_or("")
}

/// Added and removed line counts over the whole diff, straight from the ops so
/// a truncated patch still reports honest totals.
fn count_changed_lines(ops: &[DiffOp]) -> (u32, u32) {
    let mut added: u32 = 0;
    let mut removed: u32 = 0;
    for op in ops {
        match op {
            DiffOp::Equal { .. } => {}
            DiffOp::Delete { old_len, .. } => {
                removed = removed.saturating_add(clamp_u32(*old_len));
            }
            DiffOp::Insert { new_len, .. } => {
                added = added.saturating_add(clamp_u32(*new_len));
            }
            DiffOp::Replace {
                old_len, new_len, ..
            } => {
                removed = removed.saturating_add(clamp_u32(*old_len));
                added = added.saturating_add(clamp_u32(*new_len));
            }
        }
    }
    (added, removed)
}

fn count_lines(text: &str) -> u32 {
    clamp_u32(text.lines().count())
}

fn clamp_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}
