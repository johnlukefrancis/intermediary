// Path: crates/im_agent/src/bundles/ignore_rules.rs
// Description: Centralized ignore patterns for bundle building and scanning

const IGNORE_DIRS: &[&str] = &[
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

const IGNORE_FILES: &[&str] = &[".DS_Store", "Thumbs.db", ".env", ".env.local"];

pub fn should_ignore_entry(name: &str, is_directory: bool) -> bool {
    if is_directory {
        return IGNORE_DIRS.iter().any(|entry| entry == &name);
    }

    IGNORE_FILES.iter().any(|entry| entry == &name)
}
