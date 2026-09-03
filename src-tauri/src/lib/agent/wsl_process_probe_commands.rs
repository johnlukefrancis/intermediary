// Path: src-tauri/src/lib/agent/wsl_process_probe_commands.rs
// Description: In-distro probe scripts that report Intermediary agent pids and distro idleness

//! Read-only counterpart to [`super::wsl_process_control_commands`]: every builder here
//! emits a script that only *reports* — never signals. The pid detectors answer "which
//! processes is this stop responsible for" (exact binary path, port env signature, port
//! listener), and the idle probe answers "may the distro be torn down". Termination lives
//! in `wsl_process_control_commands` and `wsl_process_tree_commands`.

use super::wsl_process_control_commands::quote_bash;

pub(super) fn build_wsl_list_exact_pids_command_line(agent_bin_wsl: &str) -> String {
    let target = quote_bash(agent_bin_wsl);
    format!(
        "target={target}; deleted_target=\"$target (deleted)\"; self=$$; pids=''; if pgrep_out=$(pgrep -f \"$target\" 2>/dev/null); then pids=\"$pgrep_out\"; else rc=$?; [ \"$rc\" -eq 1 ] || exit \"$rc\"; fi; for pid in $pids; do [ \"$pid\" = \"$self\" ] && continue; exe=$(readlink \"/proc/$pid/exe\" 2>/dev/null || true); if [ \"$exe\" = \"$target\" ] || [ \"$exe\" = \"$deleted_target\" ]; then echo \"$pid\"; continue; fi; cmdline=$(tr '\\0' ' ' < \"/proc/$pid/cmdline\" 2>/dev/null || true); case \"$cmdline\" in *\"$target\"*) echo \"$pid\";; esac; done"
    )
}

pub(super) fn build_wsl_list_intermediary_agent_pids_command_line(wsl_port: u16) -> String {
    let target_port = quote_bash(&wsl_port.to_string());
    format!(
        "target_port={target_port}; self=$$; pids=''; if pgrep_out=$(pgrep -x im_agent 2>/dev/null); then pids=\"$pgrep_out\"; else rc=$?; [ \"$rc\" -eq 1 ] || exit \"$rc\"; fi; for pid in $pids; do [ \"$pid\" = \"$self\" ] && continue; env_lines=$(tr '\\0' '\\n' < \"/proc/$pid/environ\" 2>/dev/null || true); case \"\n$env_lines\n\" in *\"\nINTERMEDIARY_AGENT_PORT=$target_port\n\"*) ;; *) continue;; esac; case \"\n$env_lines\n\" in *\"\nINTERMEDIARY_WSL_WS_TOKEN=\"*) echo \"$pid\";; esac; done"
    )
}

/// Lists PIDs bound to `wsl_port` as a TCP listener (via `ss`) and confirmed to be an
/// Intermediary `im_agent` — by `comm`, executable basename, or the presence of
/// `INTERMEDIARY_WSL_WS_TOKEN` in the environment. This recognises our own backend even when
/// it was launched from a different install path or with a different token/port env, while
/// never matching a foreign non-Intermediary listener. If `ss` is unavailable the command
/// simply yields no PIDs (callers fall back to the path/env detectors).
pub(super) fn build_wsl_list_port_listener_pids_command_line(wsl_port: u16) -> String {
    let target_port = quote_bash(&wsl_port.to_string());
    format!(
        "target_port={target_port}; self=$$; \
listeners=$(ss -H -ltnp \"sport = :$target_port\" 2>/dev/null || true); \
pids=$(printf '%s\\n' \"$listeners\" | grep -oE 'pid=[0-9]+' | cut -d= -f2 | sort -u); \
for pid in $pids; do \
[ \"$pid\" = \"$self\" ] && continue; \
[ -d \"/proc/$pid\" ] || continue; \
comm=$(cat \"/proc/$pid/comm\" 2>/dev/null || true); \
if [ \"$comm\" = im_agent ]; then echo \"$pid\"; continue; fi; \
exe=$(readlink \"/proc/$pid/exe\" 2>/dev/null || true); \
case \"$exe\" in */im_agent) echo \"$pid\"; continue;; *\"/im_agent (deleted)\") echo \"$pid\"; continue;; esac; \
env_lines=$(tr '\\0' '\\n' < \"/proc/$pid/environ\" 2>/dev/null || true); \
case \"\n$env_lines\n\" in *\"\nINTERMEDIARY_WSL_WS_TOKEN=\"*) echo \"$pid\";; esac; \
done"
    )
}

/// Probes whether the distro is otherwise idle for exit-time teardown. "Busy" iff any process is
/// attached to a `pts/*` pseudo-terminal (an interactive WSL shell/tab), excluding our own agent
/// PIDs and the probe shell. Console gettys on `hvc0`/`tty1` and headless system services have no
/// `pts` and are correctly ignored. Emits `busy <distro>` or `idle <distro>` (distro from
/// `WSL_DISTRO_NAME`).
pub(super) fn build_wsl_idle_teardown_probe_command_line(agent_pids: &[u32]) -> String {
    let exclude = agent_pids
        .iter()
        .map(u32::to_string)
        .collect::<Vec<String>>()
        .join(" ");
    let exclude_quoted = quote_bash(&exclude);
    format!(
        "self=$$; exclude={exclude_quoted}; distro=${{WSL_DISTRO_NAME:-}}; busy=0; \
for pid in $(ps -eo pid=,tty= | awk '$2 ~ /^pts\\// {{ print $1 }}'); do \
[ \"$pid\" = \"$self\" ] && continue; \
skip=0; for ex in $exclude; do [ \"$pid\" = \"$ex\" ] && skip=1 && break; done; \
[ \"$skip\" = 1 ] && continue; \
busy=1; break; \
done; \
if [ \"$busy\" = 1 ]; then echo \"busy $distro\"; else echo \"idle $distro\"; fi"
    )
}

#[cfg(test)]
mod tests {
    use super::{
        build_wsl_idle_teardown_probe_command_line, build_wsl_list_exact_pids_command_line,
        build_wsl_list_intermediary_agent_pids_command_line,
        build_wsl_list_port_listener_pids_command_line,
    };

    #[test]
    fn wsl_list_exact_pids_command_targets_absolute_agent_path() {
        let command = build_wsl_list_exact_pids_command_line(
            "/mnt/c/Users/john/AppData/Local/Intermediary/agent/im_agent",
        );
        assert_eq!(
            command,
            "target='/mnt/c/Users/john/AppData/Local/Intermediary/agent/im_agent'; deleted_target=\"$target (deleted)\"; self=$$; pids=''; if pgrep_out=$(pgrep -f \"$target\" 2>/dev/null); then pids=\"$pgrep_out\"; else rc=$?; [ \"$rc\" -eq 1 ] || exit \"$rc\"; fi; for pid in $pids; do [ \"$pid\" = \"$self\" ] && continue; exe=$(readlink \"/proc/$pid/exe\" 2>/dev/null || true); if [ \"$exe\" = \"$target\" ] || [ \"$exe\" = \"$deleted_target\" ]; then echo \"$pid\"; continue; fi; cmdline=$(tr '\\0' ' ' < \"/proc/$pid/cmdline\" 2>/dev/null || true); case \"$cmdline\" in *\"$target\"*) echo \"$pid\";; esac; done"
        );
    }

    #[test]
    fn wsl_list_exact_pids_uses_cmdline_fallback_for_same_agent_path() {
        let command = build_wsl_list_exact_pids_command_line(
            "/mnt/c/Users/john/AppData/Local/Intermediary/agent/im_agent",
        );

        assert!(command.contains("cmdline=$(tr '\\0' ' ' < \"/proc/$pid/cmdline\""));
        assert!(command.contains("case \"$cmdline\" in *\"$target\"*) echo \"$pid\";; esac"));
    }

    #[test]
    fn wsl_list_intermediary_agent_pids_scopes_by_port_and_token_env() {
        let command = build_wsl_list_intermediary_agent_pids_command_line(3142);

        assert!(command.contains("pgrep -x im_agent"));
        assert!(command.contains("INTERMEDIARY_AGENT_PORT=$target_port"));
        assert!(command.contains("INTERMEDIARY_WSL_WS_TOKEN="));
        assert!(!command.contains("pkill"));
    }

    #[test]
    fn wsl_list_port_listener_pids_uses_ss_and_confirms_im_agent() {
        let command = build_wsl_list_port_listener_pids_command_line(3142);

        assert!(command.contains("ss -H -ltnp \"sport = :$target_port\""));
        assert!(command.contains("grep -oE 'pid=[0-9]+'"));
        // Confirmation signals: comm, exe basename (+ deleted), and token env — never a bare pkill.
        assert!(command.contains("[ \"$comm\" = im_agent ]"));
        assert!(command.contains("*/im_agent) echo \"$pid\""));
        assert!(command.contains("*\"/im_agent (deleted)\")"));
        assert!(command.contains("INTERMEDIARY_WSL_WS_TOKEN="));
        assert!(!command.contains("pkill"));
        assert!(!command.contains("kill "));
    }

    #[test]
    fn wsl_idle_teardown_probe_keys_on_pts_and_excludes_agent_pids() {
        let command = build_wsl_idle_teardown_probe_command_line(&[42, 99]);

        // Interactive sessions = pts/* only (agetty on hvc0/tty1 is ignored).
        assert!(command.contains("$2 ~ /^pts\\//"));
        assert!(command.contains("exclude='42 99'"));
        assert!(command.contains("distro=${WSL_DISTRO_NAME:-}"));
        assert!(command.contains("echo \"busy $distro\""));
        assert!(command.contains("echo \"idle $distro\""));
    }

    #[test]
    fn wsl_idle_teardown_probe_handles_empty_agent_pids() {
        let command = build_wsl_idle_teardown_probe_command_line(&[]);
        assert!(command.contains("exclude=''"));
    }
}
