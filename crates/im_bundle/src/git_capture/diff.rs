// Path: crates/im_bundle/src/git_capture/diff.rs
// Description: Bounded selected-path Git diff, stat, and name-status capture

use std::ffi::OsString;

use crate::cancel::BundleCancelToken;
use crate::error::Result;

use super::command::{run_git, GitCommandFailure};
use super::path::GitPath;
use super::status::ParsedStatus;
use super::{GitCaptureConfig, GitCaptureIssue, GIT_DIFF_NAME, GIT_STATUS_NAME};

const PATCH_LIMIT: usize = 32 * 1024 * 1024;
const SUMMARY_LIMIT: usize = 4 * 1024 * 1024;

#[derive(Debug, Default)]
pub(crate) struct CapturedDiffArtifacts {
    pub(crate) patch: Vec<u8>,
    pub(crate) stat: Vec<u8>,
    pub(crate) name_status: Vec<u8>,
    pub(crate) issues: Vec<GitCaptureIssue>,
}

pub(crate) fn capture_diff_artifacts(
    config: &GitCaptureConfig,
    status: &ParsedStatus,
    head_sha: &str,
    cancel_token: Option<&BundleCancelToken>,
) -> Result<CapturedDiffArtifacts> {
    let mut artifacts = CapturedDiffArtifacts::default();
    capture_group(
        config,
        head_sha,
        &status.general_pathspecs,
        false,
        &mut artifacts,
        cancel_token,
    )?;
    capture_group(
        config,
        head_sha,
        &status.rename_pathspecs,
        true,
        &mut artifacts,
        cancel_token,
    )?;
    Ok(artifacts)
}

pub(crate) fn recapture_patch(
    config: &GitCaptureConfig,
    status: &ParsedStatus,
    head_sha: &str,
    cancel_token: Option<&BundleCancelToken>,
) -> Result<std::result::Result<Vec<u8>, GitCaptureIssue>> {
    let mut patch = Vec::new();
    for (paths, detect_renames) in [
        (&status.general_pathspecs, false),
        (&status.rename_pathspecs, true),
    ] {
        if paths.is_empty() {
            continue;
        }
        match run_diff(
            config,
            DiffOutput::Patch,
            head_sha,
            paths,
            detect_renames,
            PATCH_LIMIT.saturating_sub(patch.len()),
            cancel_token,
        )? {
            Ok(output) => patch.extend_from_slice(&output),
            Err(issue) => return Ok(Err(issue)),
        }
    }
    Ok(Ok(patch))
}

fn capture_group(
    config: &GitCaptureConfig,
    head_sha: &str,
    paths: &[GitPath],
    detect_renames: bool,
    artifacts: &mut CapturedDiffArtifacts,
    cancel_token: Option<&BundleCancelToken>,
) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }
    for output_kind in [DiffOutput::Patch, DiffOutput::Stat, DiffOutput::NameStatus] {
        let (target, limit) = match output_kind {
            DiffOutput::Patch => (&mut artifacts.patch, PATCH_LIMIT),
            DiffOutput::Stat => (&mut artifacts.stat, SUMMARY_LIMIT),
            DiffOutput::NameStatus => (&mut artifacts.name_status, SUMMARY_LIMIT),
        };
        let remaining = limit.saturating_sub(target.len());
        match run_diff(
            config,
            output_kind,
            head_sha,
            paths,
            detect_renames,
            remaining,
            cancel_token,
        )? {
            Ok(output) => target.extend_from_slice(&output),
            Err(issue) => artifacts.issues.push(issue),
        }
    }
    Ok(())
}

fn run_diff(
    config: &GitCaptureConfig,
    output_kind: DiffOutput,
    head_sha: &str,
    paths: &[GitPath],
    detect_renames: bool,
    output_limit: usize,
    cancel_token: Option<&BundleCancelToken>,
) -> Result<std::result::Result<Vec<u8>, GitCaptureIssue>> {
    if output_limit == 0 {
        return Ok(Err(limit_issue(output_kind)));
    }
    let Some(mut args) = diff_args(output_kind, head_sha, paths, detect_renames) else {
        return Ok(Err(GitCaptureIssue::new(
            "unsupportedPathEncoding",
            Some(output_kind.artifact()),
            "At least one selected Git path could not be passed to Git on this host.",
        )));
    };
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiffOutput {
    Patch,
    Stat,
    NameStatus,
}

impl DiffOutput {
    fn artifact(self) -> &'static str {
        match self {
            Self::Patch => GIT_DIFF_NAME,
            Self::Stat | Self::NameStatus => GIT_STATUS_NAME,
        }
    }
}

fn diff_args(
    output_kind: DiffOutput,
    head_sha: &str,
    paths: &[GitPath],
    detect_renames: bool,
) -> Option<Vec<OsString>> {
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
        DiffOutput::Patch => args.extend([
            "--patch".into(),
            "--full-index".into(),
            "--unified=3".into(),
            "--src-prefix=a/".into(),
            "--dst-prefix=b/".into(),
        ]),
        DiffOutput::Stat => args.extend([
            "--no-patch".into(),
            "--stat=120,80".into(),
            "--summary".into(),
        ]),
        DiffOutput::NameStatus => args.push("--name-status".into()),
    }
    args.push(head_sha.into());
    args.push("--".into());
    for path in paths {
        args.push(path.to_os_string()?);
    }
    Some(args)
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

fn command_issue(output_kind: DiffOutput, failure: GitCommandFailure) -> GitCaptureIssue {
    let (kind, detail) = match failure {
        GitCommandFailure::MissingExecutable => (
            "gitUnavailable",
            "The Git executable became unavailable during evidence capture.",
        ),
        GitCommandFailure::TimedOut => (
            "commandTimeout",
            "A bounded Git evidence command timed out.",
        ),
        GitCommandFailure::SpawnFailed
        | GitCommandFailure::InputWriteFailed
        | GitCommandFailure::OutputReadFailed => (
            "commandFailure",
            "A Git evidence command could not be executed or read.",
        ),
        GitCommandFailure::NotGitRepository | GitCommandFailure::NonZeroExit => (
            "commandFailure",
            "A Git evidence command returned a non-zero status.",
        ),
    };
    GitCaptureIssue::new(kind, Some(output_kind.artifact()), detail)
}

fn limit_issue(output_kind: DiffOutput) -> GitCaptureIssue {
    GitCaptureIssue::new(
        "outputTruncated",
        Some(output_kind.artifact()),
        "The artifact output safety bound was exhausted.",
    )
}
