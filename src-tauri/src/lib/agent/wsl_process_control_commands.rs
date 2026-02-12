// Path: src-tauri/src/lib/agent/wsl_process_control_commands.rs
// Description: Shared command-line builders and quoting helpers for WSL process control

pub(super) fn build_wsl_bash_args(distro: Option<&str>, command_line: &str) -> Vec<String> {
    let mut args: Vec<String> = Vec::with_capacity(6);
    if let Some(name) = normalize_distro(distro) {
        args.push("-d".to_string());
        args.push(name);
    }

    args.push("--".to_string());
    args.push("bash".to_string());
    args.push("-lc".to_string());
    args.push(command_line.to_string());
    args
}

pub(super) fn build_wsl_spawn_command_line(
    agent_bin_wsl: &str,
    wsl_port: u16,
    wsl_ws_token: &str,
    version: &str,
    log_dir_wsl: &str,
) -> String {
    let env_port = wsl_port.to_string();
    let env_version = quote_bash(version);
    let env_log = quote_bash(log_dir_wsl);
    let env_wsl_ws_token = quote_bash(wsl_ws_token);
    let env_stdio_logging = quote_bash("0");
    let agent_bin = quote_bash(agent_bin_wsl);
    format!(
        "chmod +x {agent_bin} && INTERMEDIARY_AGENT_PORT={env_port} INTERMEDIARY_WSL_WS_TOKEN={env_wsl_ws_token} INTERMEDIARY_AGENT_VERSION={env_version} INTERMEDIARY_AGENT_LOG_DIR={env_log} INTERMEDIARY_AGENT_STDIO_LOGGING={env_stdio_logging} {agent_bin}"
    )
}

pub(super) fn build_wsl_list_exact_pids_command_line(agent_bin_wsl: &str) -> String {
    let target = quote_bash(agent_bin_wsl);
    format!(
        "target={target}; deleted_target=\"$target (deleted)\"; self=$$; pids=''; if pgrep_out=$(pgrep -f \"$target\" 2>/dev/null); then pids=\"$pgrep_out\"; else rc=$?; [ \"$rc\" -eq 1 ] || exit \"$rc\"; fi; for pid in $pids; do [ \"$pid\" = \"$self\" ] && continue; exe=$(readlink \"/proc/$pid/exe\" 2>/dev/null || true); if [ \"$exe\" = \"$target\" ] || [ \"$exe\" = \"$deleted_target\" ]; then echo \"$pid\"; fi; done"
    )
}

pub(super) fn build_wsl_signal_pids_command_line(pids: &[u32], signal: &str) -> String {
    if pids.is_empty() {
        return "true".to_string();
    }
    let pid_list = pids
        .iter()
        .map(u32::to_string)
        .collect::<Vec<String>>()
        .join(" ");
    format!("kill -{signal} {pid_list}")
}

pub(super) fn normalize_distro(distro: Option<&str>) -> Option<String> {
    distro
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

pub(super) fn distro_label(distro: Option<&str>) -> &str {
    distro
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("default")
}

fn quote_bash(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    let escaped = value.replace('\'', "'\"'\"'");
    format!("'{escaped}'")
}

#[cfg(test)]
mod tests {
    use super::{
        build_wsl_bash_args, build_wsl_list_exact_pids_command_line,
        build_wsl_signal_pids_command_line, build_wsl_spawn_command_line, normalize_distro,
    };

    #[test]
    fn normalize_distro_trims_and_filters_empty() {
        assert_eq!(normalize_distro(None), None);
        assert_eq!(normalize_distro(Some("   ")), None);
        assert_eq!(
            normalize_distro(Some("  Ubuntu-22.04  ")),
            Some("Ubuntu-22.04".to_string())
        );
    }

    #[test]
    fn wsl_list_exact_pids_command_targets_absolute_agent_path() {
        let command = build_wsl_list_exact_pids_command_line(
            "/mnt/c/Users/john/AppData/Local/Intermediary/agent/im_agent",
        );
        assert_eq!(
            command,
            "target='/mnt/c/Users/john/AppData/Local/Intermediary/agent/im_agent'; deleted_target=\"$target (deleted)\"; self=$$; pids=''; if pgrep_out=$(pgrep -f \"$target\" 2>/dev/null); then pids=\"$pgrep_out\"; else rc=$?; [ \"$rc\" -eq 1 ] || exit \"$rc\"; fi; for pid in $pids; do [ \"$pid\" = \"$self\" ] && continue; exe=$(readlink \"/proc/$pid/exe\" 2>/dev/null || true); if [ \"$exe\" = \"$target\" ] || [ \"$exe\" = \"$deleted_target\" ]; then echo \"$pid\"; fi; done"
        );
    }

    #[test]
    fn wsl_signal_command_targets_explicit_pid_list() {
        let command = build_wsl_signal_pids_command_line(&[12, 34, 56], "TERM");
        assert_eq!(command, "kill -TERM 12 34 56");
    }

    #[test]
    fn wsl_signal_command_empty_pid_list_is_noop() {
        let command = build_wsl_signal_pids_command_line(&[], "KILL");
        assert_eq!(command, "true");
    }

    #[test]
    fn wsl_spawn_command_uses_absolute_agent_path_not_relative_invocation() {
        let command = build_wsl_spawn_command_line(
            "/mnt/c/Users/john/AppData/Local/Intermediary/agent/im_agent",
            3142,
            "token-123",
            "1.2.3",
            "/mnt/c/Users/john/AppData/Local/Intermediary/logs",
        );

        assert!(command
            .contains("chmod +x '/mnt/c/Users/john/AppData/Local/Intermediary/agent/im_agent'"));
        assert!(!command.contains("./im_agent"));
        assert!(command.contains("INTERMEDIARY_AGENT_STDIO_LOGGING='0'"));
        assert!(command.ends_with("'/mnt/c/Users/john/AppData/Local/Intermediary/agent/im_agent'"));
    }

    #[test]
    fn wsl_bash_args_include_distro_when_set() {
        let args = build_wsl_bash_args(Some("  Ubuntu-22.04 "), "echo hi");
        assert_eq!(
            args,
            vec![
                "-d".to_string(),
                "Ubuntu-22.04".to_string(),
                "--".to_string(),
                "bash".to_string(),
                "-lc".to_string(),
                "echo hi".to_string(),
            ]
        );
    }
}
