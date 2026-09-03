// Path: crates/im_bundle/src/git_capture/porcelain.rs
// Description: Strict parser for NUL-delimited Git porcelain-v2 records

use std::path::Path;

use crate::selection::SelectedPathKind;

use super::path::{bytes_to_path, display_ref, strip_repo_prefix, GitPath};

/// Parsed `git status --porcelain=v2 -z --branch` output. `branch` is `None`
/// when HEAD is detached; `head_sha` is `None` on an unborn branch; the
/// upstream fields are `None` when no upstream is configured.
pub struct PorcelainStatus {
    pub head_sha: Option<String>,
    pub branch: Option<String>,
    pub upstream: Option<String>,
    pub ahead: Option<u64>,
    pub behind: Option<u64>,
    pub records: Vec<StatusRecord>,
}

#[derive(Debug)]
pub struct StatusRecord {
    pub xy: String,
    pub current: GitPath,
    pub original: Option<GitPath>,
    head_mode: Vec<u8>,
    worktree_mode: Vec<u8>,
    pub score: Option<String>,
    record_type: u8,
}

impl StatusRecord {
    pub fn is_untracked(&self) -> bool {
        self.record_type == b'?'
    }

    pub fn is_unmerged(&self) -> bool {
        self.record_type == b'u'
    }

    pub fn is_deleted(&self) -> bool {
        self.xy.as_bytes().contains(&b'D')
    }

    /// Octal mode of the HEAD-side entry as Git printed it (`000000` when absent).
    pub fn head_mode(&self) -> &[u8] {
        &self.head_mode
    }

    /// Octal mode of the worktree-side entry as Git printed it (`000000` when absent).
    pub fn worktree_mode(&self) -> &[u8] {
        &self.worktree_mode
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

pub fn parse_porcelain(output: &[u8]) -> std::result::Result<PorcelainStatus, String> {
    let mut fields = output.split(|byte| *byte == 0).peekable();
    let mut headers = BranchHeaders::default();
    let mut records = Vec::new();

    while let Some(field) = fields.next() {
        if field.is_empty() {
            continue;
        }
        match field.first().copied() {
            Some(b'#') => parse_header(field, &mut headers),
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
        head_sha: headers.head_sha,
        branch: headers.branch,
        upstream: headers.upstream,
        ahead: headers.ahead,
        behind: headers.behind,
        records,
    })
}

#[derive(Default)]
struct BranchHeaders {
    head_sha: Option<String>,
    branch: Option<String>,
    upstream: Option<String>,
    ahead: Option<u64>,
    behind: Option<u64>,
}

fn parse_header(field: &[u8], headers: &mut BranchHeaders) {
    if let Some(value) = field.strip_prefix(b"# branch.oid ") {
        if value != b"(initial)" {
            headers.head_sha = String::from_utf8(value.to_vec()).ok();
        }
    } else if let Some(value) = field.strip_prefix(b"# branch.head ") {
        if value != b"(detached)" {
            headers.branch = Some(display_ref(value));
        }
    } else if let Some(value) = field.strip_prefix(b"# branch.upstream ") {
        headers.upstream = Some(display_ref(value));
    } else if let Some(value) = field.strip_prefix(b"# branch.ab ") {
        // `# branch.ab +<ahead> -<behind>`
        let text = String::from_utf8_lossy(value);
        let mut parts = text.split_whitespace();
        let ahead = parts.next().and_then(|part| part.strip_prefix('+')).and_then(|n| n.parse().ok());
        let behind = parts.next().and_then(|part| part.strip_prefix('-')).and_then(|n| n.parse().ok());
        headers.ahead = ahead;
        headers.behind = behind;
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
