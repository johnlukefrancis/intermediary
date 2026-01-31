// Path: crates/im_bundle/src/ignore_rules.rs
// Description: Always-ignored file and directory names for bundle scanning

const IGNORED_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "dist",
    "build",
    "target",
    ".next",
    ".cache",
    "logs",
    ".turbo",
    ".vercel",
    "__pycache__",
    ".mypy_cache",
    ".pytest_cache",
    "coverage",
];

const IGNORED_FILES: &[&str] = &[".DS_Store", "Thumbs.db", ".env", ".env.local"];

pub fn is_ignored_dir(name: &str) -> bool {
    IGNORED_DIRS.contains(&name)
}

pub fn is_ignored_file(name: &str) -> bool {
    IGNORED_FILES.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignore_rules_match() {
        assert!(is_ignored_dir("node_modules"));
        assert!(is_ignored_dir(".git"));
        assert!(is_ignored_file(".env"));
        assert!(!is_ignored_file("README.md"));
    }
}
