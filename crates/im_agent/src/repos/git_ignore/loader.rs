// Path: crates/im_agent/src/repos/git_ignore/loader.rs
// Description: Gathers a repo's Git ignore files (tracked .gitignore, info/exclude, core.excludesFile) off the async runtime

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::Duration;

use im_bundle::git::{
    bytes_to_path, common_git_config_args, run_git_with_input, trim_line_ending, KillPolicy,
};

use super::rules::{GitIgnoreRules, IgnoreSources};

const GIT_EXECUTABLE: &str = "git";
const GIT_TIMEOUT: Duration = Duration::from_secs(10);
const STDOUT_LIMIT: usize = 4 * 1024 * 1024;
const ROOT_GITIGNORE: &str = ".gitignore";
/// `git config --get-regexp` exits 1 when no key matches.
const CONFIG_UNSET_CODES: &[i32] = &[1];

/// Rules live in Git's top-level coordinates; `prefix` maps the watched root
/// (possibly a subfolder of the repository) into them.
pub(crate) struct LoadedRules {
    rules: GitIgnoreRules,
    prefix: String,
    /// Absolute as the watcher reports it, so its change events can be recognized.
    pub(crate) exclude_path: PathBuf,
}

impl LoadedRules {
    /// `relative` is root-relative with `/` separators.
    pub(crate) fn is_ignored(&self, relative: &str, is_dir: bool) -> bool {
        if self.prefix.is_empty() {
            return self.rules.is_ignored(relative, is_dir);
        }
        self.rules
            .is_ignored(&format!("{}{relative}", self.prefix), is_dir)
    }
}

struct RepoLocation {
    top_level: PathBuf,
    prefix: String,
    exclude_path: PathBuf,
}

/// `extra_gitignores` are root-relative `.gitignore` paths seen in watcher
/// events, so untracked ignore files count without walking the tree.
pub(crate) async fn load_rules(
    root: &Path,
    extra_gitignores: Vec<String>,
) -> Result<LoadedRules, String> {
    let root = root.to_path_buf();
    tokio::task::spawn_blocking(move || load_blocking(&root, extra_gitignores))
        .await
        .map_err(|err| format!("git ignore load task failed: {err}"))?
}

fn load_blocking(root: &Path, extra_gitignores: Vec<String>) -> Result<LoadedRules, String> {
    let listed = git_stdout(
        root,
        "ls-files",
        &["ls-files", "-z", "--", ROOT_GITIGNORE, "*/.gitignore"],
        &[],
    )?;
    let location = locate(root)?;
    // Keyed by top-level-relative path; the root's ancestors up to the top
    // level come first, then the root's tracked and event-seen files.
    let mut files: BTreeSet<String> = ancestor_gitignores(&location.prefix);
    files.extend(
        listed
            .split(|byte| *byte == 0)
            .filter(|chunk| !chunk.is_empty())
            .filter_map(bytes_to_path)
            .map(|path| path.to_string_lossy().replace('\\', "/"))
            .chain(extra_gitignores)
            .map(|relative| format!("{}{relative}", location.prefix)),
    );

    let config = git_stdout(
        root,
        "config",
        &[
            "config",
            "-z",
            "--type=path",
            "--get-regexp",
            r"^core\.(excludesfile|ignorecase)$",
        ],
        CONFIG_UNSET_CODES,
    )?;
    let (excludes_file, case_insensitive) = parse_config(&config);

    let mut sources = IgnoreSources {
        case_insensitive,
        ..IgnoreSources::default()
    };
    for file in files {
        if let Some(contents) = read_optional(&location.top_level.join(&file))? {
            let dir = file.rsplit_once('/').map_or("", |(dir, _)| dir);
            sources.per_directory.push((dir.to_string(), contents));
        }
    }
    let global_files = [
        Some(location.exclude_path.clone()),
        excludes_file.or_else(default_excludes_file),
    ];
    for path in global_files.into_iter().flatten() {
        if let Some(contents) = read_optional(&path)? {
            sources.global.push(contents);
        }
    }

    Ok(LoadedRules {
        rules: GitIgnoreRules::build(&sources)?,
        prefix: location.prefix,
        exclude_path: location.exclude_path,
    })
}

fn locate(root: &Path) -> Result<RepoLocation, String> {
    let stdout = git_stdout(
        root,
        "rev-parse",
        &[
            "rev-parse",
            "--show-toplevel",
            "--show-prefix",
            "--git-path",
            "info/exclude",
        ],
        &[],
    )?;
    // One value per line; `--show-prefix` prints an empty line at the top level.
    let lines: Vec<Vec<u8>> = stdout
        .split(|byte| *byte == b'\n')
        .map(|line| trim_line_ending(line.to_vec()))
        .collect();
    let path_at = |index: usize, what: &str| {
        lines
            .get(index)
            .filter(|line| !line.is_empty())
            .and_then(|line| bytes_to_path(line))
            .ok_or_else(|| format!("git rev-parse returned no {what}"))
    };
    let top_level = path_at(0, "top level")?;
    let prefix = lines
        .get(1)
        .map(|line| String::from_utf8_lossy(line).replace('\\', "/"))
        .ok_or_else(|| "git rev-parse returned no prefix".to_string())?;
    let exclude_path = root.join(path_at(2, "info/exclude path")?);
    Ok(RepoLocation {
        top_level,
        prefix,
        exclude_path,
    })
}

/// `.gitignore` in the top level and every directory down to the root itself.
fn ancestor_gitignores(prefix: &str) -> BTreeSet<String> {
    let mut files = BTreeSet::from([ROOT_GITIGNORE.to_string()]);
    for (end, _) in prefix.match_indices('/') {
        files.insert(format!("{}/{ROOT_GITIGNORE}", &prefix[..end]));
    }
    files
}

fn git_stdout(
    root: &Path,
    label: &str,
    args: &[&str],
    accepted_codes: &[i32],
) -> Result<Vec<u8>, String> {
    let mut full = common_git_config_args();
    full.extend(args.iter().map(OsString::from));
    run_git_with_input(
        Path::new(GIT_EXECUTABLE),
        root,
        &full,
        None,
        accepted_codes,
        STDOUT_LIMIT,
        GIT_TIMEOUT,
        None,
        KillPolicy::Immediate,
    )
    .map_err(|err| format!("git {label} failed: {err}"))?
    .map(|output| output.stdout)
    .map_err(|failure| {
        format!(
            "git {label} failed: {:?} {}",
            failure.kind,
            failure.message()
        )
    })
}

/// `-z` records are `key\nvalue`; keys arrive lower-cased.
fn parse_config(stdout: &[u8]) -> (Option<PathBuf>, bool) {
    let mut excludes_file = None;
    let mut case_insensitive = false;
    for record in stdout.split(|byte| *byte == 0) {
        let text = String::from_utf8_lossy(record);
        let Some((key, value)) = text.split_once('\n') else {
            continue;
        };
        match key {
            "core.excludesfile" if !value.is_empty() => excludes_file = Some(PathBuf::from(value)),
            "core.ignorecase" => {
                case_insensitive = matches!(
                    value.to_ascii_lowercase().as_str(),
                    "true" | "yes" | "on" | "1"
                )
            }
            _ => {}
        }
    }
    (excludes_file, case_insensitive)
}

/// Git's fallback when `core.excludesFile` is unset.
fn default_excludes_file() -> Option<PathBuf> {
    if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(config_home).join("git").join("ignore"));
    }
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .filter(|home| !home.is_empty())
        .map(|home| {
            PathBuf::from(home)
                .join(".config")
                .join("git")
                .join("ignore")
        })
}

fn read_optional(path: &Path) -> Result<Option<String>, String> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(String::from_utf8_lossy(&bytes).into_owned())),
        Err(err) if matches!(err.kind(), ErrorKind::NotFound | ErrorKind::NotADirectory) => {
            Ok(None)
        }
        Err(err) => Err(format!("read {} failed: {err}", path.display())),
    }
}
