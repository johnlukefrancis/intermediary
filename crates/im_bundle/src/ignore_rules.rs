// Path: crates/im_bundle/src/ignore_rules.rs
// Description: Always-ignored file and directory names for bundle scanning

const IGNORED_DIRS: &[&str] = &[
    ".cache",
    ".git",
    ".huggingface",
    ".mypy_cache",
    ".next",
    ".nyc_output",
    ".nuxt",
    ".parcel-cache",
    ".pytest_cache",
    ".ruff_cache",
    ".svelte-kit",
    ".tox",
    ".turbo",
    ".venv",
    ".gradle",
    ".hypothesis",
    ".nox",
    "__pycache__",
    "build",
    "checkpoints",
    "coverage",
    "dist",
    "env",
    "huggingface_hub",
    "logs",
    "mlruns",
    "models",
    "node_modules",
    "out",
    "target",
    "venv",
    "wandb",
    "weights",
];

const IGNORED_DIR_SUFFIXES: &[&str] = &[".egg-info"];

const IGNORED_FILES: &[&str] = &[
    ".DS_Store",
    ".coverage",
    ".env",
    ".env.local",
    ".eslintcache",
    "Thumbs.db",
    "thumbs.db",
];

const IGNORED_FILE_SUFFIXES: &[&str] = &[
    ".bak",
    ".bin",
    ".ckpt",
    ".h5",
    ".keras",
    ".log",
    ".onnx",
    ".old",
    ".orig",
    ".pb",
    ".pt",
    ".pth",
    ".pyc",
    ".pyd",
    ".pyo",
    ".safetensors",
    ".swo",
    ".swp",
    ".tmp",
    "~",
];

pub fn is_ignored_dir(name: &str) -> bool {
    if IGNORED_DIRS.contains(&name) {
        return true;
    }
    IGNORED_DIR_SUFFIXES
        .iter()
        .any(|suffix| name.ends_with(suffix))
}

pub fn is_ignored_file(name: &str) -> bool {
    if IGNORED_FILES.contains(&name) {
        return true;
    }
    IGNORED_FILE_SUFFIXES
        .iter()
        .any(|suffix| name.ends_with(suffix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignore_rules_match() {
        assert!(is_ignored_dir("node_modules"));
        assert!(is_ignored_dir(".git"));
        assert!(is_ignored_dir("package.egg-info"));
        assert!(is_ignored_dir("wandb"));
        assert!(is_ignored_file(".env"));
        assert!(is_ignored_file("debug.log"));
        assert!(is_ignored_file("module.pyc"));
        assert!(is_ignored_file("model.safetensors"));
        assert!(!is_ignored_file("README.md"));
    }
}
