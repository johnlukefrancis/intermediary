// Path: crates/im_agent/src/repos/ignore_matcher.rs
// Description: Ignore glob matcher for repo watcher

use globset::{Glob, GlobBuilder, GlobSet, GlobSetBuilder};

use crate::error::AgentError;

const DEFAULT_IGNORE_PATTERNS: &[&str] = &[
    "**/node_modules/**",
    "**/.git/**",
    "**/dist/**",
    "**/build/**",
    "**/target/**",
    "**/*.log",
    "**/logs/**",
    "**/.DS_Store",
    "**/Thumbs.db",
];

pub(crate) struct IgnoreMatcher {
    set: GlobSet,
}

impl IgnoreMatcher {
    pub(crate) fn new(extra_globs: &[String]) -> Result<Self, AgentError> {
        let mut builder = GlobSetBuilder::new();
        for pattern in DEFAULT_IGNORE_PATTERNS.iter() {
            builder.add(build_glob(pattern)?);
        }
        for pattern in extra_globs {
            builder.add(build_glob(pattern)?);
        }
        let set = builder
            .build()
            .map_err(|err| AgentError::new("INVALID_GLOB", err.to_string()))?;
        Ok(Self { set })
    }

    pub(crate) fn should_ignore(&self, relative_path: &str) -> bool {
        self.set.is_match(relative_path)
    }
}

fn build_glob(pattern: &str) -> Result<Glob, AgentError> {
    GlobBuilder::new(pattern)
        .case_insensitive(true)
        .literal_separator(false)
        .backslash_escape(false)
        .build()
        .map_err(|err| AgentError::new("INVALID_GLOB", err.to_string()))
}
