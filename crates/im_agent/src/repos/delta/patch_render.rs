// Path: crates/im_agent/src/repos/delta/patch_render.rs
// Description: Renders diff ops as hunks-only unified patch text - per-row length cap, PATCH_MAX_BYTES budget, removed rows omitted for a fingerprint baseline

use similar::udiff::UnifiedHunkHeader;
use similar::{group_diff_ops, DiffOp};

use crate::protocol::DeltaStats;

use super::{PatchOutput, CONTEXT_RADIUS, PATCH_LINE_MAX_BYTES, PATCH_MAX_BYTES};

/// Where removed rows come from.
pub(super) enum OldRows<'a> {
    Text(&'a [&'a str]),
    /// A fingerprint baseline kept no text: each hunk's header still counts its
    /// removed rows, and a header note says how many were not retained.
    Omitted,
}

/// Renders whole hunks until the next would pass `PATCH_MAX_BYTES`. Stats are
/// measured over every op, so a cut or row-capped patch still reports exact totals.
pub(super) fn render_patch(ops: Vec<DiffOp>, old: &OldRows<'_>, new: &[&str]) -> PatchOutput {
    let (added, removed) = count_changed_lines(&ops);
    let mut patch = String::new();
    let mut hunks: u32 = 0;
    let mut truncated = false;
    let mut budget_spent = false;
    for group in group_diff_ops(ops, CONTEXT_RADIUS) {
        hunks = hunks.saturating_add(1);
        if budget_spent {
            continue;
        }
        let (rendered, partial) = render_hunk(&group, old, new);
        truncated |= partial;
        if patch.len().saturating_add(rendered.len()) <= PATCH_MAX_BYTES {
            patch.push_str(&rendered);
            continue;
        }
        budget_spent = true;
        truncated = true;
        if patch.is_empty() {
            // One hunk wider than the whole budget: keep its whole-line prefix.
            patch.push_str(line_prefix(&rendered, PATCH_MAX_BYTES));
        }
    }
    PatchOutput {
        patch,
        stats: DeltaStats {
            added,
            removed,
            hunks,
            new_lines: clamp_u32(new.len()),
        },
        truncated,
    }
}

/// One hunk: header, then context and changed rows in `similar`'s order
/// (removed before added). True when a row was capped or removed text omitted.
fn render_hunk(group: &[DiffOp], old: &OldRows<'_>, new: &[&str]) -> (String, bool) {
    let mut body = String::new();
    let mut partial = false;
    let mut omitted: usize = 0;
    for op in group {
        let (_, old_range, new_range) = op.as_tag_tuple();
        match op {
            DiffOp::Equal { .. } => partial |= push_rows(&mut body, ' ', rows(new, new_range)),
            DiffOp::Insert { .. } => partial |= push_rows(&mut body, '+', rows(new, new_range)),
            DiffOp::Delete { .. } | DiffOp::Replace { .. } => {
                match old {
                    OldRows::Text(old) => {
                        partial |= push_rows(&mut body, '-', rows(old, old_range));
                    }
                    OldRows::Omitted => omitted = omitted.saturating_add(old_range.len()),
                }
                if matches!(op, DiffOp::Replace { .. }) {
                    partial |= push_rows(&mut body, '+', rows(new, new_range));
                }
            }
        }
    }
    let mut hunk = UnifiedHunkHeader::new(group).to_string();
    if omitted > 0 {
        let noun = if omitted == 1 { "line" } else { "lines" };
        hunk.push_str(&format!(" {omitted} removed {noun} not retained"));
    }
    hunk.push('\n');
    hunk.push_str(&body);
    (hunk, partial || omitted > 0)
}

fn rows<'a>(lines: &'a [&'a str], range: std::ops::Range<usize>) -> &'a [&'a str] {
    lines.get(range).unwrap_or(&[])
}

fn push_rows(out: &mut String, prefix: char, lines: &[&str]) -> bool {
    let mut capped = false;
    for line in lines {
        capped |= push_row(out, prefix, line);
    }
    capped
}

/// Appends one row with its terminator stripped and its text cut to
/// `PATCH_LINE_MAX_BYTES` on a char boundary; true when the cut happened.
pub(super) fn push_row(out: &mut String, prefix: char, line: &str) -> bool {
    let body = line.strip_suffix('\n').unwrap_or(line);
    let body = body.strip_suffix('\r').unwrap_or(body);
    let mut end = body.len().min(PATCH_LINE_MAX_BYTES);
    while !body.is_char_boundary(end) {
        end -= 1;
    }
    out.push(prefix);
    out.push_str(body.get(..end).unwrap_or(""));
    out.push('\n');
    end < body.len()
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

/// Added and removed line counts over the whole diff, straight from the ops.
fn count_changed_lines(ops: &[DiffOp]) -> (u32, u32) {
    let mut added: u32 = 0;
    let mut removed: u32 = 0;
    for op in ops {
        let (_, old_range, new_range) = op.as_tag_tuple();
        match op {
            DiffOp::Equal { .. } => {}
            DiffOp::Delete { .. } => removed = removed.saturating_add(clamp_u32(old_range.len())),
            DiffOp::Insert { .. } => added = added.saturating_add(clamp_u32(new_range.len())),
            DiffOp::Replace { .. } => {
                removed = removed.saturating_add(clamp_u32(old_range.len()));
                added = added.saturating_add(clamp_u32(new_range.len()));
            }
        }
    }
    (added, removed)
}

pub(super) fn clamp_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}
