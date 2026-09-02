// Path: crates/im_bundle/src/git_capture/diff.rs
// Description: Bounded selected-path Git diff, stat, and name-status capture

use std::ffi::OsString;

use crate::cancel::BundleCancelToken;
use crate::error::Result;

use super::command::run_git;
use super::diff_issue::{command_issue, limit_issue, pathspec_issue, DiffOutput};
use super::pathspec_batches::{batch_rename_pairs, batch_single_paths, PathspecBatchError};
use super::status::ParsedStatus;
use super::{GitCaptureConfig, GitCaptureIssue, PatchDeletions};

pub(super) const PATCH_LIMIT: usize = 32 * 1024 * 1024;
pub(super) const FULL_DELETIONS_BUDGET: usize = 8 * 1024 * 1024;
const SUMMARY_LIMIT: usize = 4 * 1024 * 1024;

#[derive(Debug, Default)]
pub(crate) struct CapturedDiffArtifacts {
    pub(crate) patch: Vec<u8>,
    pub(crate) index_patch: Vec<u8>,
    pub(crate) worktree_patch: Vec<u8>,
    pub(crate) stat: Vec<u8>,
    pub(crate) name_status: Vec<u8>,
    pub(crate) issues: Vec<GitCaptureIssue>,
}

pub(crate) fn capture_diff_artifacts(
    config: &GitCaptureConfig,
    status: &ParsedStatus,
    head_sha: &str,
    deletions: PatchDeletions,
    cancel_token: Option<&BundleCancelToken>,
) -> Result<CapturedDiffArtifacts> {
    let mut artifacts = CapturedDiffArtifacts::default();
    let general_batches = batch_single_paths(&status.general_pathspecs);
    capture_group(
        config,
        head_sha,
        &general_batches,
        false,
        deletions,
        &mut artifacts,
        cancel_token,
    )?;
    let rename_batches = batch_rename_pairs(&status.rename_pathspecs);
    capture_group(
        config,
        head_sha,
        &rename_batches,
        true,
        deletions,
        &mut artifacts,
        cancel_token,
    )?;
    Ok(artifacts)
}

pub(crate) fn recapture_patch(
    config: &GitCaptureConfig,
    status: &ParsedStatus,
    head_sha: &str,
    deletions: PatchDeletions,
    cancel_token: Option<&BundleCancelToken>,
) -> Result<std::result::Result<Vec<u8>, GitCaptureIssue>> {
    let mut patch = Vec::new();
    for (batches, detect_renames) in [
        (batch_single_paths(&status.general_pathspecs), false),
        (batch_rename_pairs(&status.rename_pathspecs), true),
    ] {
        let batches = match batches {
            Ok(batches) => batches,
            Err(error) => return Ok(Err(pathspec_issue(DiffOutput::Patch, error))),
        };
        let (output, issue) = run_diff_batches(
            config,
            DiffOutput::Patch,
            head_sha,
            &batches,
            detect_renames,
            deletions,
            config.patch_limit.saturating_sub(patch.len()),
            cancel_token,
        )?;
        patch.extend_from_slice(&output);
        if let Some(issue) = issue {
            return Ok(Err(issue));
        }
    }
    Ok(Ok(patch))
}

fn capture_group(
    config: &GitCaptureConfig,
    head_sha: &str,
    batches: &std::result::Result<Vec<Vec<OsString>>, PathspecBatchError>,
    detect_renames: bool,
    deletions: PatchDeletions,
    artifacts: &mut CapturedDiffArtifacts,
    cancel_token: Option<&BundleCancelToken>,
) -> Result<()> {
    for output_kind in [
        DiffOutput::Patch,
        DiffOutput::IndexPatch,
        DiffOutput::WorktreePatch,
        DiffOutput::Stat,
        DiffOutput::NameStatus,
    ] {
        let (target, limit) = match output_kind {
            DiffOutput::Patch => (&mut artifacts.patch, config.patch_limit),
            DiffOutput::IndexPatch => (&mut artifacts.index_patch, config.patch_limit),
            DiffOutput::WorktreePatch => (&mut artifacts.worktree_patch, config.patch_limit),
            DiffOutput::Stat => (&mut artifacts.stat, SUMMARY_LIMIT),
            DiffOutput::NameStatus => (&mut artifacts.name_status, SUMMARY_LIMIT),
        };
        let remaining = limit.saturating_sub(target.len());
        let (output, issue) = match batches {
            Ok(batches) => run_diff_batches(
                config,
                output_kind,
                head_sha,
                batches,
                detect_renames,
                deletions,
                remaining,
                cancel_token,
            )?,
            Err(error) => (Vec::new(), Some(pathspec_issue(output_kind, *error))),
        };
        target.extend_from_slice(&output);
        if let Some(issue) = issue {
            artifacts.issues.push(issue);
        }
    }
    Ok(())
}

fn run_diff_batches(
    config: &GitCaptureConfig,
    output_kind: DiffOutput,
    head_sha: &str,
    batches: &[Vec<OsString>],
    detect_renames: bool,
    deletions: PatchDeletions,
    output_limit: usize,
    cancel_token: Option<&BundleCancelToken>,
) -> Result<(Vec<u8>, Option<GitCaptureIssue>)> {
    let mut captured = Vec::new();
    for paths in batches {
        let remaining = output_limit.saturating_sub(captured.len());
        match run_diff(
            config,
            output_kind,
            head_sha,
            paths,
            detect_renames,
            deletions,
            remaining,
            cancel_token,
        )? {
            Ok(output) => captured.extend_from_slice(&output),
            Err(issue) => return Ok((captured, Some(issue))),
        }
    }
    Ok((captured, None))
}

fn run_diff(
    config: &GitCaptureConfig,
    output_kind: DiffOutput,
    head_sha: &str,
    paths: &[OsString],
    detect_renames: bool,
    deletions: PatchDeletions,
    output_limit: usize,
    cancel_token: Option<&BundleCancelToken>,
) -> Result<std::result::Result<Vec<u8>, GitCaptureIssue>> {
    if output_limit == 0 {
        return Ok(Err(limit_issue(output_kind)));
    }
    let mut args = diff_args(output_kind, head_sha, paths, detect_renames, deletions);
    let result = run_git(
        &config.executable,
        &config.repo_root,
        &args,
        output_limit,
        config.command_timeout,
        cancel_token,
    )?;
    args.clear();
    let output = match result {
        Ok(output) => output,
        Err(failure) => {
            return Ok(Err(command_issue(output_kind, failure)));
        }
    };
    if output.stdout_truncated {
        return Ok(Err(GitCaptureIssue::new(
            "outputTruncated",
            Some(output_kind.artifact()),
            "The Git command reached its documented output safety bound; this artifact is incomplete.",
        )));
    }
    Ok(Ok(output.stdout))
}

fn diff_args(
    output_kind: DiffOutput,
    head_sha: &str,
    paths: &[OsString],
    detect_renames: bool,
    deletions: PatchDeletions,
) -> Vec<OsString> {
    let mut args = common_git_args();
    args.push("diff".into());
    args.extend([
        "--no-ext-diff".into(),
        "--no-textconv".into(),
        "--no-color".into(),
        "--submodule=short".into(),
        "--relative".into(),
        "--diff-algorithm=myers".into(),
        if detect_renames {
            "--find-renames".into()
        } else {
            "--no-renames".into()
        },
    ]);
    match output_kind {
        DiffOutput::Patch | DiffOutput::IndexPatch | DiffOutput::WorktreePatch => {
            if output_kind == DiffOutput::IndexPatch {
                args.push("--cached".into());
            }
            args.extend([
                "--patch".into(),
                "--full-index".into(),
                "--unified=3".into(),
                "--src-prefix=a/".into(),
                "--dst-prefix=b/".into(),
            ]);
            if deletions == PatchDeletions::HeaderOnly {
                args.push("--irreversible-delete".into());
            }
        }
        DiffOutput::Stat => args.extend([
            "--no-patch".into(),
            "--stat=120,80".into(),
            "--summary".into(),
        ]),
        DiffOutput::NameStatus => args.push("--name-status".into()),
    }
    // The worktree patch compares index to worktree, so it names no commit.
    if output_kind != DiffOutput::WorktreePatch {
        args.push(head_sha.into());
    }
    args.push("--".into());
    args.extend(paths.iter().cloned());
    args
}

pub(super) fn common_git_args() -> Vec<OsString> {
    let mut args = vec![OsString::from("--literal-pathspecs")];
    args.extend(common_git_config_args());
    args
}

pub(super) fn common_git_config_args() -> Vec<OsString> {
    [
        "-c",
        "core.fsmonitor=false",
        "-c",
        "core.untrackedCache=false",
        "-c",
        "core.quotePath=true",
        "-c",
        "color.ui=false",
    ]
    .into_iter()
    .map(OsString::from)
    .collect()
}
