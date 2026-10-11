// Path: crates/im_agent/src/repos/delta/unified_patch.rs
// Description: Hunks-only unified patch text and exact stats for one delta, against a text or fingerprint baseline

use std::time::Instant;

use similar::{capture_diff_slices_deadline, Algorithm};

use crate::protocol::DeltaStats;

use super::baseline::{fingerprint, split_lines};
use super::patch_render::{clamp_u32, push_row, render_patch, OldRows};
use super::{Baseline, PATCH_MAX_BYTES};

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
#[cfg(test)]
pub(crate) fn compute_patch(old: &str, new: &str, deadline: Instant) -> PatchOutput {
    let old_lines = split_lines(old);
    let new_lines = split_lines(new);
    diff_text_lines(&old_lines, &new_lines, deadline)
}

/// Diffs `new` against either baseline form. A fingerprint baseline compares
/// line hashes, so added and context rows are real text and removed rows are counted only.
pub(crate) fn diff_against(
    old: &Baseline,
    new_lines: &[&str],
    new_prints: Option<&[u64]>,
    deadline: Instant,
) -> PatchOutput {
    match old {
        Baseline::Text(text) => diff_text_lines(&split_lines(text), new_lines, deadline),
        Baseline::Lines(old_prints) => {
            let computed;
            let new_prints = match new_prints {
                Some(prints) => prints,
                None => {
                    computed = new_lines
                        .iter()
                        .map(|line| fingerprint(line))
                        .collect::<Vec<_>>();
                    &computed
                }
            };
            let ops = capture_diff_slices_deadline(
                Algorithm::Myers,
                old_prints,
                new_prints,
                Some(deadline),
            );
            render_patch(ops, &OldRows::Omitted, new_lines)
        }
    }
}

fn diff_text_lines(old: &[&str], new: &[&str], deadline: Instant) -> PatchOutput {
    let ops = capture_diff_slices_deadline(Algorithm::Myers, old, new, Some(deadline));
    render_patch(ops, &OldRows::Text(old), new)
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

/// The deletion card for a fingerprint baseline: the exact removed count in a
/// header-only hunk whose note says the text was not retained.
pub(crate) fn uncounted_removal_patch(lines: usize) -> PatchOutput {
    let rows = clamp_u32(lines);
    if rows == 0 {
        return all_removed_patch("");
    }
    let noun = if rows == 1 { "line" } else { "lines" };
    PatchOutput {
        patch: format!("@@ -1,{rows} +0,0 @@ {rows} removed {noun} not retained\n"),
        stats: DeltaStats {
            added: 0,
            removed: rows,
            hunks: 1,
            new_lines: 0,
        },
        truncated: true,
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

/// One prefix per row, stopping at the last whole row inside `PATCH_MAX_BYTES`; the header
/// counts emitted rows, budgeted at the full count (fewer rows never lengthen it).
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
    let mut row = String::new();
    let mut rows: u32 = 0;
    let mut truncated = false;
    for line in text.lines() {
        row.clear();
        let capped = push_row(&mut row, prefix, line);
        if body.len().saturating_add(row.len()) > budget {
            truncated = true;
            break;
        }
        body.push_str(&row);
        truncated |= capped;
        rows = rows.saturating_add(1);
    }
    if rows == 0 {
        // Unreachable while a capped row fits the budget; a bare header would
        // claim rows it does not carry, so the card would rest on its stats.
        return (String::new(), true);
    }
    let mut patch = header_for(rows);
    patch.push_str(&body);
    (patch, truncated)
}

fn count_lines(text: &str) -> u32 {
    clamp_u32(text.lines().count())
}
