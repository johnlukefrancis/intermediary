// Path: crates/im_bundle/src/git_capture/render.rs
// Description: Selection-safe human-readable Git status and bundle handoff artifacts

use super::status::{SelectedRecordKind, SelectedStatusRecord};
use super::{BundleGitCapture, GitCaptureState};

pub(crate) fn render_status(
    capture: &BundleGitCapture,
    records: &[SelectedStatusRecord],
    diff_stat: &[u8],
    name_status: &[u8],
) -> Vec<u8> {
    let mut output = String::new();
    output.push_str("Intermediary captured Git working-tree evidence\n");
    output.push_str("===============================================\n\n");
    output.push_str(&format!("Capture status: {}\n", state_name(capture.status)));
    output.push_str(&format!("Captured at: {}\n", capture.captured_at));
    output.push_str("Comparison base: HEAD\n");
    output.push_str(&format!(
        "Captured HEAD: {}\n",
        capture.head_sha.as_deref().unwrap_or("unavailable")
    ));
    output.push_str(&format!(
        "Branch: {}\n",
        capture
            .branch
            .as_deref()
            .unwrap_or("detached or unavailable")
    ));
    output.push_str(&format!(
        "Repository dirty: {}\n",
        optional_bool(capture.repo_dirty)
    ));
    output.push_str(&format!(
        "Bundled selection dirty: {}\n",
        optional_bool(capture.selection_dirty)
    ));
    output.push_str(&format!(
        "Changed paths omitted by bundle selection: {}\n",
        capture
            .counts
            .omitted_changed_paths
            .map(|count| count.to_string())
            .unwrap_or_else(|| "unavailable".to_string())
    ));

    output.push_str("\nSelected-path counts\n--------------------\n");
    output.push_str(&format!(
        "changed={} tracked={} untracked={} deleted={} renamed={} conflicted={}\n",
        capture.counts.selected_changed,
        capture.counts.selected_tracked_changed,
        capture.counts.selected_untracked,
        capture.counts.selected_deleted,
        capture.counts.selected_renamed,
        capture.counts.selected_conflicted,
    ));

    output.push_str("\nSelected staging/worktree status\n--------------------------------\n");
    if records.is_empty() {
        if capture.status == GitCaptureState::Complete && capture.selection_dirty == Some(false) {
            output.push_str("Clean: no selected path differs from captured HEAD.\n");
        } else {
            output.push_str("No selected status records are available.\n");
        }
    } else {
        for record in records {
            render_record(&mut output, record);
        }
    }

    output.push_str("\nSelected diff stat\n------------------\n");
    append_git_output(
        &mut output,
        diff_stat,
        "(empty: no selected tracked delta)\n",
    );
    output.push_str("\nSelected diff name-status\n-------------------------\n");
    append_git_output(
        &mut output,
        name_status,
        "(empty: no selected tracked delta)\n",
    );

    if capture.counts.selected_untracked > 0 {
        output.push_str(
            "\nSelected untracked files are ordinary bundled files with no HEAD ancestor; their contents are not duplicated into BUNDLE_GIT_DIFF.patch.\n",
        );
    }
    if capture.counts.omitted_changed_paths.unwrap_or(0) > 0 {
        output.push_str(
            "\nThe repository has additional changed paths excluded by bundle selection. Their names and contents are intentionally not reproduced here.\n",
        );
    }
    if !capture.issues.is_empty() {
        output.push_str("\nCapture issues\n--------------\n");
        for issue in &capture.issues {
            output.push_str("- ");
            output.push_str(&issue.kind);
            if let Some(artifact) = &issue.artifact {
                output.push_str(" [");
                output.push_str(artifact);
                output.push(']');
            }
            output.push_str(": ");
            output.push_str(&issue.detail);
            output.push('\n');
        }
    }
    output.into_bytes()
}

pub(crate) fn handoff_note() -> &'static [u8] {
    br#"# Intermediary bundle handoff

This archive is captured repository evidence, not a mutable Git repository.

Read in this order:

1. `BUNDLE_MANIFEST.json` for the bundle contract, selection, and capture quality.
2. `BUNDLE_GIT_STATUS.txt` for branch/HEAD orientation and selected-path state.
3. `BUNDLE_GIT_DIFF.patch` for the selected tracked working-tree delta from captured HEAD.
4. When present, `docs/guide.md`, the recent portion of `docs/changelog.md`, then the relevant source and documentation.

Do not expect a `.git` directory or ask for live Git commands after upload. Treat generated Git artifacts as evidence captured during bundle construction and honor any partial, unavailable, or unstable status.

When giving operator instructions, use repository-local paths. Never tell an operator to open, edit, or run commands against this bundle archive.
"#
}

fn render_record(output: &mut String, record: &SelectedStatusRecord) {
    output.push_str(&record.xy);
    output.push(' ');
    match record.kind {
        SelectedRecordKind::Renamed => match (&record.original, &record.current) {
            (Some(original), Some(current)) => {
                output.push_str(record.score.as_deref().unwrap_or("R"));
                output.push(' ');
                output.push_str(&original.display());
                output.push_str(" -> ");
                output.push_str(&current.display());
            }
            (Some(original), None) => {
                output.push_str(&original.display());
                output.push_str(" [renamed counterpart omitted by selection]");
            }
            (None, Some(current)) => {
                output.push_str(&current.display());
                output.push_str(" [renamed counterpart omitted by selection]");
            }
            (None, None) => output.push_str("[unavailable selected rename path]"),
        },
        SelectedRecordKind::Untracked => {
            if let Some(path) = &record.current {
                output.push_str(&path.display());
            }
            output.push_str(" [untracked; no HEAD ancestor]");
        }
        SelectedRecordKind::IgnoredUntracked => {
            if let Some(path) = &record.current {
                output.push_str(&path.display());
            }
            output.push_str(" [untracked; ignored by Git; no HEAD ancestor]");
        }
        SelectedRecordKind::Unmerged => {
            if let Some(path) = &record.current {
                output.push_str(&path.display());
            }
            output.push_str(" [conflicted]");
        }
        SelectedRecordKind::Changed => {
            if let Some(path) = &record.current {
                output.push_str(&path.display());
            }
        }
    }
    if record.counterpart_omitted && record.kind != SelectedRecordKind::Renamed {
        output.push_str(" [counterpart omitted by selection]");
    }
    output.push('\n');
}

fn append_git_output(output: &mut String, bytes: &[u8], empty: &str) {
    if bytes.is_empty() {
        output.push_str(empty);
        return;
    }
    output.push_str(&String::from_utf8_lossy(bytes));
    if !bytes.ends_with(b"\n") {
        output.push('\n');
    }
}

fn optional_bool(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "yes",
        Some(false) => "no",
        None => "unavailable",
    }
}

fn state_name(state: GitCaptureState) -> &'static str {
    match state {
        GitCaptureState::Complete => "complete",
        GitCaptureState::Partial => "partial",
        GitCaptureState::Unavailable => "unavailable",
        GitCaptureState::Unstable => "unstable",
    }
}
