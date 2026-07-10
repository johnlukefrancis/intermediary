// Path: crates/im_bundle/src/git_capture/porcelain.rs
// Description: Strict parser for NUL-delimited Git porcelain-v2 records

use std::path::Path;

use crate::selection::SelectedPathKind;

use super::path::{bytes_to_path, display_ref, strip_repo_prefix, GitPath};

pub(crate) struct PorcelainStatus {
    pub(crate) head_sha: Option<String>,
    pub(crate) branch: Option<String>,
    pub(crate) records: Vec<StatusRecord>,
}

#[derive(Debug)]
pub(crate) struct StatusRecord {
    pub(crate) xy: String,
    pub(crate) current: GitPath,
    pub(crate) original: Option<GitPath>,
    head_mode: Vec<u8>,
    worktree_mode: Vec<u8>,
    pub(crate) score: Option<String>,
    record_type: u8,
}

impl StatusRecord {
    pub(crate) fn is_untracked(&self) -> bool {
        self.record_type == b'?'
    }

    pub(crate) fn is_unmerged(&self) -> bool {
        self.record_type == b'u'
    }

    pub(crate) fn is_deleted(&self) -> bool {
        self.xy.as_bytes().contains(&b'D')
    }

    pub(crate) fn current_kind(&self, repo_root: &Path, repo_prefix: &[u8]) -> SelectedPathKind {
        if self.is_untracked() {
            return strip_repo_prefix(self.current.as_bytes(), repo_prefix)
                .and_then(bytes_to_path)
                .and_then(|path| std::fs::symlink_metadata(repo_root.join(path)).ok())
                .map(|metadata| {
                    if metadata.file_type().is_symlink() {
                        SelectedPathKind::Symlink
                    } else {
                        SelectedPathKind::File
                    }
                })
                .unwrap_or(SelectedPathKind::File);
        }
        mode_kind(if self.worktree_mode == b"000000" {
            &self.head_mode
        } else {
            &self.worktree_mode
        })
    }

    pub(crate) fn original_kind(&self) -> SelectedPathKind {
        mode_kind(&self.head_mode)
    }
}

pub(crate) fn parse_porcelain(output: &[u8]) -> std::result::Result<PorcelainStatus, String> {
    let mut fields = output.split(|byte| *byte == 0).peekable();
    let mut head_sha = None;
    let mut branch = None;
    let mut records = Vec::new();

    while let Some(field) = fields.next() {
        if field.is_empty() {
            continue;
        }
        match field.first().copied() {
            Some(b'#') => parse_header(field, &mut head_sha, &mut branch),
            Some(b'1') => records.push(parse_ordinary(field)?),
            Some(b'2') => {
                let original = fields
                    .next()
                    .ok_or_else(|| "rename status record omitted its original path".to_string())?;
                records.push(parse_rename(field, original)?);
            }
            Some(b'u') => records.push(parse_unmerged(field)?),
            Some(b'?') => records.push(parse_untracked(field)?),
            Some(other) => {
                return Err(format!("unsupported porcelain-v2 record type {other}"));
            }
            None => {}
        }
    }
    Ok(PorcelainStatus {
        head_sha,
        branch,
        records,
    })
}

fn parse_header(field: &[u8], head_sha: &mut Option<String>, branch: &mut Option<String>) {
    if let Some(value) = field.strip_prefix(b"# branch.oid ") {
        if value != b"(initial)" {
            *head_sha = String::from_utf8(value.to_vec()).ok();
        }
    } else if let Some(value) = field.strip_prefix(b"# branch.head ") {
        if value != b"(detached)" {
            *branch = Some(display_ref(value));
        }
    }
}

fn parse_ordinary(field: &[u8]) -> std::result::Result<StatusRecord, String> {
    let values = split_fields(field, 9, "ordinary")?;
    Ok(StatusRecord {
        xy: ascii(&values[1]),
        current: GitPath::from_bytes(&values[8]),
        original: None,
        head_mode: values[3].clone(),
        worktree_mode: values[5].clone(),
        score: None,
        record_type: b'1',
    })
}

fn parse_rename(field: &[u8], original: &[u8]) -> std::result::Result<StatusRecord, String> {
    let values = split_fields(field, 10, "rename")?;
    Ok(StatusRecord {
        xy: ascii(&values[1]),
        current: GitPath::from_bytes(&values[9]),
        original: Some(GitPath::from_bytes(original)),
        head_mode: values[3].clone(),
        worktree_mode: values[5].clone(),
        score: Some(ascii(&values[8])),
        record_type: b'2',
    })
}

fn parse_unmerged(field: &[u8]) -> std::result::Result<StatusRecord, String> {
    let values = split_fields(field, 11, "unmerged")?;
    Ok(StatusRecord {
        xy: ascii(&values[1]),
        current: GitPath::from_bytes(&values[10]),
        original: None,
        head_mode: values[3].clone(),
        worktree_mode: values[6].clone(),
        score: None,
        record_type: b'u',
    })
}

fn parse_untracked(field: &[u8]) -> std::result::Result<StatusRecord, String> {
    let path = field
        .strip_prefix(b"? ")
        .ok_or_else(|| "invalid untracked status record".to_string())?;
    Ok(StatusRecord {
        xy: "??".to_string(),
        current: GitPath::from_bytes(path),
        original: None,
        head_mode: b"000000".to_vec(),
        worktree_mode: b"000000".to_vec(),
        score: None,
        record_type: b'?',
    })
}

fn split_fields(
    field: &[u8],
    count: usize,
    kind: &str,
) -> std::result::Result<Vec<Vec<u8>>, String> {
    let values: Vec<Vec<u8>> = field
        .splitn(count, |byte| *byte == b' ')
        .map(|value| value.to_vec())
        .collect();
    if values.len() != count {
        return Err(format!("invalid {kind} status record"));
    }
    Ok(values)
}

fn mode_kind(mode: &[u8]) -> SelectedPathKind {
    match mode {
        b"120000" => SelectedPathKind::Symlink,
        b"160000" => SelectedPathKind::DirectoryLike,
        _ => SelectedPathKind::File,
    }
}

fn ascii(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}
