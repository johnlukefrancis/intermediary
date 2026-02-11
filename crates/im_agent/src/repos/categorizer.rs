// Path: crates/im_agent/src/repos/categorizer.rs
// Description: File kind classification based on globs and fallback heuristics

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};

use super::generated_code_extensions::GENERATED_CODE_EXTENSIONS;
use crate::error::AgentError;
use crate::protocol::FileKind;

const DOC_EXTENSIONS: &[&str] = &[".md", ".txt", ".rst", ".adoc", ".asciidoc", ".wiki"];
const DOC_IMAGE_EXTENSIONS: &[&str] = &[
    ".png", ".jpg", ".jpeg", ".webp", ".gif", ".bmp", ".heic", ".heif", ".tiff", ".tif",
];

const DOC_DIRS: &[&str] = &["docs", "doc", "documentation", "wiki"];

const CODE_FILE_NAMES: &[&str] = &[
    "makefile",
    "dockerfile",
    "containerfile",
    "cmakelists.txt",
    "meson.build",
    "meson_options.txt",
];

#[derive(Debug, Clone)]
pub struct Categorizer {
    docs: Option<GlobSet>,
    code: Option<GlobSet>,
}

impl Categorizer {
    pub fn new(docs_globs: &[String], code_globs: &[String]) -> Result<Self, AgentError> {
        let docs = build_glob_set(docs_globs)?;
        let code = build_glob_set(code_globs)?;

        Ok(Self { docs, code })
    }

    pub fn categorize(&self, relative_path: &str) -> FileKind {
        let normalized = relative_path.replace('\\', "/");

        if let Some(docs) = &self.docs {
            if docs.is_match(&normalized) {
                return FileKind::Docs;
            }
        }

        if let Some(code) = &self.code {
            if code.is_match(&normalized) {
                return FileKind::Code;
            }
        }

        fallback_categorize(&normalized)
    }
}

fn build_glob_set(globs: &[String]) -> Result<Option<GlobSet>, AgentError> {
    if globs.is_empty() {
        return Ok(None);
    }

    let mut builder = GlobSetBuilder::new();
    for glob in globs {
        let compiled = GlobBuilder::new(glob)
            .case_insensitive(true)
            .literal_separator(false)
            .backslash_escape(false)
            .build()
            .map_err(|err| AgentError::new("INVALID_GLOB", err.to_string()))?;
        builder.add(compiled);
    }

    builder
        .build()
        .map(Some)
        .map_err(|err| AgentError::new("INVALID_GLOB", err.to_string()))
}

fn fallback_categorize(relative_path: &str) -> FileKind {
    let lower = relative_path.to_lowercase();
    let parts: Vec<&str> = lower.split('/').collect();
    let file_name = parts.last().copied().unwrap_or_default();

    for part in parts.iter().take(parts.len().saturating_sub(1)) {
        if DOC_DIRS.iter().any(|dir| dir == part) {
            return FileKind::Docs;
        }
    }

    if let Some(ext) = file_name.rsplit_once('.') {
        let ext = format!(".{}", ext.1);
        if DOC_EXTENSIONS.contains(&ext.as_str()) {
            return FileKind::Docs;
        }
        if DOC_IMAGE_EXTENSIONS.contains(&ext.as_str()) {
            return FileKind::Docs;
        }
        if GENERATED_CODE_EXTENSIONS.contains(&ext.as_str()) {
            return FileKind::Code;
        }
    }

    if CODE_FILE_NAMES.contains(&file_name) {
        return FileKind::Code;
    }

    if file_name == "readme" || file_name.starts_with("readme.") {
        return FileKind::Docs;
    }

    FileKind::Other
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_classifies_cpp_family_as_code() {
        assert_eq!(fallback_categorize("Test.cpp"), FileKind::Code);
        assert_eq!(fallback_categorize("Source/GameMode.hpp"), FileKind::Code);
        assert_eq!(fallback_categorize("Source/LinuxEntry.cc"), FileKind::Code);
        assert_eq!(fallback_categorize("Source/Platform.mm"), FileKind::Code);
    }

    #[test]
    fn fallback_keeps_docs_priority() {
        assert_eq!(fallback_categorize("docs/notes.txt"), FileKind::Docs);
        assert_eq!(fallback_categorize("README.md"), FileKind::Docs);
    }

    #[test]
    fn fallback_classifies_images_as_docs() {
        assert_eq!(fallback_categorize("Screenshots/Capture.png"), FileKind::Docs);
        assert_eq!(fallback_categorize("captures/IMG_0001.JPG"), FileKind::Docs);
        assert_eq!(fallback_categorize("notes/snip.webp"), FileKind::Docs);
    }
}
