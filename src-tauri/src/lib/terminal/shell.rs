// Path: src-tauri/src/lib/terminal/shell.rs
// Description: Profile-faithful PowerShell command and exact inherited environment for terminal spawn

use super::start_dir::StartDir;
#[cfg(not(windows))]
use portable_pty::CommandBuilder;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[cfg(windows)]
const PWSH_DEFAULT: &str = r"C:\Program Files\PowerShell\7\pwsh.exe";

const INHERITED_TERMINAL_KEYS: [&str; 3] = ["TERM_PROGRAM_VERSION", "WT_SESSION", "WT_PROFILE_ID"];

#[derive(Debug, Clone)]
pub struct TerminalCommand {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub cwd: PathBuf,
    pub env: Vec<(OsString, OsString)>,
}

impl TerminalCommand {
    #[cfg(not(windows))]
    pub fn into_portable(self) -> CommandBuilder {
        let mut command = CommandBuilder::new(self.program);
        command.args(self.args);
        command.cwd(self.cwd);
        command.env_clear();
        for (key, value) in self.env {
            command.env(key, value);
        }
        command
    }
}

#[cfg(windows)]
pub fn resolve_pwsh() -> Result<PathBuf, String> {
    let default = PathBuf::from(PWSH_DEFAULT);
    if default.is_file() {
        return Ok(default);
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&path)
        .map(|dir| dir.join("pwsh.exe"))
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| {
            "PowerShell 7 (pwsh.exe) was not found in Program Files or on PATH".to_string()
        })
}

#[cfg(not(windows))]
pub fn resolve_pwsh() -> Result<PathBuf, String> {
    Err("The integrated terminal is available on Windows hosts only".to_string())
}

/// Builds `pwsh -NoLogo [-NoExit -Command <entry>]` without `-NoProfile`.
pub fn build_command(pwsh: &Path, start: &StartDir) -> TerminalCommand {
    let mut args = vec![OsString::from("-NoLogo")];
    if let Some(entry) = &start.initial_command {
        args.extend([
            OsString::from("-NoExit"),
            OsString::from("-Command"),
            OsString::from(entry),
        ]);
    }
    TerminalCommand {
        program: pwsh.to_path_buf(),
        args,
        cwd: start.cwd.clone(),
        env: terminal_environment(std::env::vars_os()),
    }
}

fn terminal_environment(
    inherited: impl IntoIterator<Item = (OsString, OsString)>,
) -> Vec<(OsString, OsString)> {
    let mut env = inherited
        .into_iter()
        .filter(|(key, _)| {
            let name = key.to_string_lossy();
            !INHERITED_TERMINAL_KEYS
                .iter()
                .any(|blocked| name.eq_ignore_ascii_case(blocked))
                && !name.to_ascii_uppercase().starts_with("VSCODE_")
                && !name.eq_ignore_ascii_case("TERM_PROGRAM")
                && !name.eq_ignore_ascii_case("COLORTERM")
        })
        .collect::<Vec<_>>();
    env.push((
        OsString::from("TERM_PROGRAM"),
        OsString::from("Intermediary"),
    ));
    env.push((OsString::from("COLORTERM"), OsString::from("truecolor")));
    env
}

#[cfg(test)]
mod tests {
    use super::{build_command, terminal_environment};
    use crate::terminal::start_dir::StartDir;
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};

    #[test]
    fn host_start_preserves_profile_and_directory() {
        let command = build_command(
            Path::new("/opt/pwsh"),
            &StartDir {
                cwd: PathBuf::from("/tmp/repo"),
                initial_command: None,
            },
        );
        assert_eq!(command.program, PathBuf::from("/opt/pwsh"));
        assert_eq!(command.args, [OsString::from("-NoLogo")]);
        assert_eq!(command.cwd, PathBuf::from("/tmp/repo"));
    }

    #[test]
    fn wsl_start_runs_the_guarded_entry_after_the_profile() {
        let command = build_command(
            Path::new("/opt/pwsh"),
            &StartDir {
                cwd: PathBuf::from("/tmp/home"),
                initial_command: Some("wsl.exe -d 'Ubuntu' --cd '/home/j'".to_string()),
            },
        );
        assert_eq!(
            command.args,
            [
                "-NoLogo",
                "-NoExit",
                "-Command",
                "wsl.exe -d 'Ubuntu' --cd '/home/j'"
            ]
            .map(OsString::from)
        );
    }

    #[test]
    fn inherited_terminal_identity_is_replaced_case_insensitively() {
        let env = terminal_environment([
            (OsString::from("wt_session"), OsString::from("abc")),
            (OsString::from("Vscode_Pid"), OsString::from("42")),
            (OsString::from("PATH"), OsString::from("kept")),
        ]);
        assert!(env
            .iter()
            .any(|(key, value)| key == "PATH" && value == "kept"));
        assert!(!env.iter().any(|(key, _)| key == "wt_session"));
        assert!(!env.iter().any(|(key, _)| key == "Vscode_Pid"));
        assert!(env
            .iter()
            .any(|(key, value)| key == "TERM_PROGRAM" && value == "Intermediary"));
    }
}
