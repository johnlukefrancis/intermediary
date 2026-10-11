// Path: crates/im_agent/src/repos/git_ignore/rules.rs
// Description: Compiled Git ignore rules with Git's per-directory precedence and excluded-parent semantics

use std::collections::HashMap;
use std::path::Path;

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use ignore::Match;

/// Raw ignore-file contents gathered by the loader; `per_directory` keys are
/// repo-relative slash dirs (`""` is the root).
#[derive(Debug, Default)]
pub(crate) struct IgnoreSources {
    pub(crate) case_insensitive: bool,
    pub(crate) per_directory: Vec<(String, String)>,
    /// Highest precedence first: `info/exclude`, then `core.excludesFile`.
    pub(crate) global: Vec<String>,
}

pub(crate) struct GitIgnoreRules {
    per_directory: HashMap<String, Gitignore>,
    global: Vec<Gitignore>,
}

impl GitIgnoreRules {
    pub(crate) fn build(sources: &IgnoreSources) -> Result<Self, String> {
        let case_insensitive = sources.case_insensitive;
        let mut per_directory = HashMap::with_capacity(sources.per_directory.len());
        for (dir, contents) in &sources.per_directory {
            per_directory.insert(dir.clone(), compile(contents, case_insensitive)?);
        }
        let global = sources
            .global
            .iter()
            .map(|contents| compile(contents, case_insensitive))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            per_directory,
            global,
        })
    }

    /// `relative` is a repo-relative slash path; an excluded parent directory
    /// excludes everything beneath it, as Git never re-includes from inside it.
    pub(crate) fn is_ignored(&self, relative: &str, is_dir: bool) -> bool {
        let relative = relative.trim_matches('/');
        if relative.is_empty() {
            return false;
        }
        let parents_excluded = relative
            .match_indices('/')
            .any(|(end, _)| self.excludes(&relative[..end], true));
        parents_excluded || self.excludes(relative, is_dir)
    }

    /// Deepest applicable `.gitignore` decides first, then the global files.
    fn excludes(&self, path: &str, is_dir: bool) -> bool {
        let mut split = path.rfind('/');
        loop {
            let (dir, rest) = match split {
                Some(end) => (&path[..end], &path[end + 1..]),
                None => ("", path),
            };
            if let Some(decided) = self
                .per_directory
                .get(dir)
                .and_then(|rules| decision(rules, rest, is_dir))
            {
                return decided;
            }
            match split {
                Some(end) => split = path[..end].rfind('/'),
                None => break,
            }
        }
        self.global
            .iter()
            .find_map(|rules| decision(rules, path, is_dir))
            .unwrap_or(false)
    }
}

fn decision(rules: &Gitignore, path: &str, is_dir: bool) -> Option<bool> {
    match rules.matched(Path::new(path), is_dir) {
        Match::Ignore(_) => Some(true),
        Match::Whitelist(_) => Some(false),
        Match::None => None,
    }
}

/// Rooted at `.` so the crate strips nothing; callers pass paths already
/// relative to the ignore file's directory.
fn compile(contents: &str, case_insensitive: bool) -> Result<Gitignore, String> {
    let mut builder = GitignoreBuilder::new(".");
    builder
        .case_insensitive(case_insensitive)
        .map_err(|err| format!("git ignore case setting failed: {err}"))?;
    for line in contents.lines() {
        // Git skips a pattern it cannot parse and keeps the rest of the file.
        let _ = builder.add_line(None, line);
    }
    builder
        .build()
        .map_err(|err| format!("git ignore rules failed to compile: {err}"))
}
