// Path: src-tauri/src/lib/wsl_control.rs
// Description: Shared bounded non-login WSL stdin-script boundary and native-root validation

use std::io::Write;
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const COMMAND_TIMEOUT: Duration = Duration::from_secs(5);
const COMMAND_POLL: Duration = Duration::from_millis(25);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedWslRoot {
    pub distro: String,
    pub path: String,
}

/// Runs a bounded control script through a non-login bash. Script bytes travel
/// over stdin, never through wsl.exe's argument marshalling.
pub fn run_wsl_script(distro: Option<&str>, script: &str) -> Result<Output, String> {
    let mut command = Command::new("wsl.exe");
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
        .args(build_bash_stdin_args(distro))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|err| {
        format!(
            "Failed to execute WSL control command (distro={}): {err}",
            distro_label(distro)
        )
    })?;

    let feed_result = match child.stdin.take() {
        Some(mut stdin) => stdin.write_all(script.as_bytes()).map_err(|err| {
            format!(
                "Failed to write WSL control script (distro={}): {err}",
                distro_label(distro)
            )
        }),
        None => Err("Failed to open WSL control command stdin".to_string()),
    };
    if let Err(err) = feed_result {
        terminate_and_observe(&mut child);
        return Err(err);
    }

    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                return child.wait_with_output().map_err(|err| {
                    format!(
                        "Failed to read WSL control output (distro={}): {err}",
                        distro_label(distro)
                    )
                });
            }
            Ok(None) if started.elapsed() < COMMAND_TIMEOUT => thread::sleep(COMMAND_POLL),
            Ok(None) => return timeout(child, distro),
            Err(err) => {
                terminate_and_observe(&mut child);
                return Err(format!(
                    "Failed to poll WSL control command (distro={}): {err}",
                    distro_label(distro)
                ));
            }
        }
    }
}

/// Resolves the selected (or actual default) distro and proves that the exact
/// native Linux path is a directory there. The returned distro is pinned for
/// the interactive wsl.exe entry.
pub fn resolve_native_root(
    configured_distro: Option<&str>,
    path: &str,
) -> Result<ResolvedWslRoot, String> {
    validate_native_path(path)?;
    let distro = match normalize_distro(configured_distro) {
        Some(distro) => distro,
        None => resolve_default_distro()?,
    };
    let output = run_wsl_script(Some(&distro), &validation_script(path, &distro))?;
    if !output.status.success() {
        let detail = sanitize_stream_text(&String::from_utf8_lossy(&output.stderr));
        let suffix = if detail.is_empty() {
            String::new()
        } else {
            format!(": {detail}")
        };
        return Err(format!(
            "Terminal WSL root is unavailable: path={path} distro={distro} exit={}{}",
            exit_label(&output.status),
            suffix
        ));
    }
    Ok(ResolvedWslRoot {
        distro,
        path: path.to_string(),
    })
}

pub fn build_bash_stdin_args(distro: Option<&str>) -> Vec<String> {
    let mut args = Vec::with_capacity(7);
    if let Some(name) = normalize_distro(distro) {
        args.push("-d".to_string());
        args.push(name);
    }
    args.extend([
        "--".to_string(),
        "bash".to_string(),
        "--noprofile".to_string(),
        "--norc".to_string(),
        "-s".to_string(),
    ]);
    args
}

pub fn normalize_distro(distro: Option<&str>) -> Option<String> {
    distro
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

pub fn distro_label(distro: Option<&str>) -> &str {
    distro
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("default")
}

pub fn sanitize_stream_text(text: &str) -> String {
    text.trim()
        .replace('\r', "")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

fn validate_native_path(path: &str) -> Result<(), String> {
    if !path.starts_with('/') || path.contains(['\0', '\r', '\n']) {
        return Err(format!(
            "Terminal WSL root must be an absolute single-line Linux path: {path}"
        ));
    }
    Ok(())
}

fn resolve_default_distro() -> Result<String, String> {
    let output = run_wsl_script(
        None,
        "if [ -z \"${WSL_DISTRO_NAME:-}\" ]; then exit 70; fi\nprintf '%s\\n' \"$WSL_DISTRO_NAME\"\n",
    )?;
    if !output.status.success() {
        return Err(format!(
            "Failed to resolve the default WSL distro (exit={}): {}",
            exit_label(&output.status),
            sanitize_stream_text(&String::from_utf8_lossy(&output.stderr))
        ));
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .next_back()
        .map(ToString::to_string)
        .ok_or_else(|| "Failed to resolve the default WSL distro: no identity returned".to_string())
}

fn validation_script(path: &str, distro: &str) -> String {
    let quoted_path = quote_bash(path);
    let quoted_distro = quote_bash(distro);
    format!(
        "set -u\nroot={quoted_path}\nexpected={quoted_distro}\nif [ \"${{WSL_DISTRO_NAME:-}}\" != \"$expected\" ]; then printf '%s\\n' 'distro identity mismatch' >&2; exit 70; fi\nif [ ! -d \"$root\" ]; then printf '%s\\n' 'path is not a directory' >&2; exit 71; fi\n"
    )
}

fn quote_bash(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn exit_label(status: &std::process::ExitStatus) -> String {
    status
        .code()
        .map(|code| code.to_string())
        .unwrap_or_else(|| "signal".to_string())
}

fn timeout(mut child: Child, distro: Option<&str>) -> Result<Output, String> {
    terminate_and_observe(&mut child);
    Err(format!(
        "WSL control command timed out after {}ms (distro={})",
        COMMAND_TIMEOUT.as_millis(),
        distro_label(distro)
    ))
}

fn terminate_and_observe(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(test)]
mod tests {
    use super::{build_bash_stdin_args, validation_script};

    #[test]
    fn stdin_runner_is_non_login_and_pins_a_named_distro() {
        assert_eq!(
            build_bash_stdin_args(Some(" Ubuntu ")),
            ["-d", "Ubuntu", "--", "bash", "--noprofile", "--norc", "-s"]
        );
    }

    #[test]
    fn validation_quotes_the_exact_path_as_script_data() {
        let script = validation_script("/home/john/it's here", "Ubuntu");
        assert!(script.contains("root='/home/john/it'\"'\"'s here'"));
        assert!(script.contains("expected='Ubuntu'"));
        assert!(script.contains("[ ! -d \"$root\" ]"));
        assert!(script.contains("${WSL_DISTRO_NAME:-}"));
    }
}
