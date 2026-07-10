// Path: crates/im_bundle/src/git_capture/path.rs
// Description: Lossless Git path transport and model-readable quoting helpers

use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct GitPath(Vec<u8>);

impl GitPath {
    pub(crate) fn from_bytes(bytes: &[u8]) -> Self {
        Self(bytes.to_vec())
    }

    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub(crate) fn to_path_buf(&self) -> Option<PathBuf> {
        bytes_to_path(&self.0)
    }

    pub(crate) fn to_os_string(&self) -> Option<OsString> {
        bytes_to_os_string(&self.0)
    }

    pub(crate) fn display(&self) -> String {
        quote_path(&self.0)
    }
}

pub(crate) fn strip_repo_prefix<'a>(path: &'a [u8], prefix: &[u8]) -> Option<&'a [u8]> {
    if prefix.is_empty() {
        return Some(path);
    }
    path.strip_prefix(prefix)
        .filter(|relative| !relative.is_empty())
}

pub(crate) fn bytes_to_path(bytes: &[u8]) -> Option<PathBuf> {
    bytes_to_os_string(bytes).map(PathBuf::from)
}

#[cfg(unix)]
pub(crate) fn path_to_bytes(path: &Path) -> Option<Vec<u8>> {
    use std::os::unix::ffi::OsStrExt;
    Some(path.as_os_str().as_bytes().to_vec())
}

#[cfg(not(unix))]
pub(crate) fn path_to_bytes(path: &Path) -> Option<Vec<u8>> {
    path.to_str().map(|value| value.as_bytes().to_vec())
}

pub(crate) fn display_ref(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap_or_else(|_| quote_path(bytes))
}

fn quote_path(bytes: &[u8]) -> String {
    if let Ok(value) = std::str::from_utf8(bytes) {
        if value
            .chars()
            .all(|character| !character.is_control() && character != '\\' && character != '"')
        {
            return value.to_string();
        }
    }
    let mut output = String::from("\"");
    for byte in bytes {
        match byte {
            b'\\' => output.push_str("\\\\"),
            b'"' => output.push_str("\\\""),
            b'\n' => output.push_str("\\n"),
            b'\r' => output.push_str("\\r"),
            b'\t' => output.push_str("\\t"),
            0x20..=0x7e => output.push(char::from(*byte)),
            _ => output.push_str(&format!("\\x{byte:02x}")),
        }
    }
    output.push('"');
    output
}

#[cfg(unix)]
fn bytes_to_os_string(bytes: &[u8]) -> Option<OsString> {
    use std::os::unix::ffi::OsStringExt;
    Some(OsString::from_vec(bytes.to_vec()))
}

#[cfg(not(unix))]
fn bytes_to_os_string(bytes: &[u8]) -> Option<OsString> {
    String::from_utf8(bytes.to_vec()).ok().map(OsString::from)
}
