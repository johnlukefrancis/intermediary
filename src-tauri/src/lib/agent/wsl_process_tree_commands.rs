// Path: src-tauri/src/lib/agent/wsl_process_tree_commands.rs
// Description: The in-distro script that kills a WSL agent's descendant process groups, then the agent

//! The WSL agent runs every Git command in its own Unix process group
//! (`im_bundle::git_capture::command_tree`), so a hook, `ssh`, or credential
//! helper it started is reachable only by that group — never by the agent's pid.
//! When the supervisor's drain envelope expires the agent is past its own
//! emergency bound and can no longer take those groups with it, so this script
//! is what does: one `ps` snapshot, a breadth-first walk of each agent's
//! descendants, `kill -KILL` to every descendant group, then to the descendants
//! that share the agent's own group, and only then to the agent itself.
//!
//! A snapshot is enough because admission is already closed: nothing new is
//! being started by the time this runs. What it cannot reach is a descendant
//! that started its own session (`setsid`), which by definition left the agent's
//! tree — an accepted boundary, recorded in `docs/known_issues.md`.
//!
//! The agent's *own* process group is deliberately never signalled as a group:
//! it is the group `wsl.exe` put the launcher shell in, and it can hold
//! processes this supervisor does not own. Descendants sharing it are killed one
//! pid at a time instead.

use super::wsl_process_control_commands::quote_bash;

/// Reads `pid ppid pgid` rows and prints the kill plan, one `<what> <id>` line
/// at a time: `group <pgid>` for a descendant group of its own, `pid <pid>` for
/// a descendant inside the agent's group, `agent <pid>` for the agent last.
/// Written for POSIX awk (mawk is the Ubuntu default) — no arrays-of-arrays, no
/// `delete`, and no single quote anywhere, because the whole program is passed
/// inside a single-quoted shell word.
const KILL_PLAN_AWK: &str = r#"
BEGIN { wanted = split(agents, want, " ") }
{
  pid = $1 + 0; ppid = $2 + 0; pgid = $3 + 0
  seen[pid] = 1
  group[pid] = pgid
  kids[ppid] = kids[ppid] " " pid
}
END {
  head = 1
  tail = 0
  for (i = 1; i <= wanted; i++) {
    a = want[i] + 0
    if (a <= 1) continue
    target[a] = 1
    if (!(a in seen)) continue
    direct_count = split(kids[a], direct, " ")
    for (j = 1; j <= direct_count; j++) {
      tail++
      queue[tail] = direct[j] + 0
      anchor[tail] = group[a]
    }
  }
  while (head <= tail) {
    d = queue[head]
    own = anchor[head]
    head++
    if (d <= 1 || (d in visited) || (d in target)) continue
    visited[d] = 1
    dg = group[d]
    if (dg > 1 && dg != own) { groups[dg] = 1 } else { lone[d] = 1 }
    child_count = split(kids[d], child, " ")
    for (j = 1; j <= child_count; j++) {
      tail++
      queue[tail] = child[j] + 0
      anchor[tail] = own
    }
  }
  for (g in groups) print "group " g
  for (p in lone) print "pid " p
  for (t in target) print "agent " t
}
"#;

/// The prefix every signalled line carries, so the supervisor can count what the
/// sweep actually reached without parsing the ids back.
const SIGNALLED_PREFIX: &str = "signalled ";

/// Builds the one script the emergency stop runs inside the distro. Fed to
/// `bash --noprofile --norc -s` over stdin like every other control script here,
/// so its newlines and quotes never cross `wsl.exe`'s argument marshalling.
pub(super) fn build_wsl_kill_agent_process_trees_command_line(agent_pids: &[u32]) -> String {
    let agents = agent_pids
        .iter()
        .map(u32::to_string)
        .collect::<Vec<String>>()
        .join(" ");
    let agents_quoted = quote_bash(&agents);
    // `pipefail` is what makes a missing or failing `ps` an error rather than an
    // empty plan that would silently signal nothing.
    format!(
        "set -uo pipefail; agents={agents_quoted}; \
plan=$(ps -e -o pid=,ppid=,pgid= | awk -v agents=\"$agents\" '{KILL_PLAN_AWK}') || exit 3; \
printf '%s\\n' \"$plan\" | while read -r what id; do \
case \"$what\" in \
group) kill -KILL -- \"-$id\" 2>/dev/null && echo \"{SIGNALLED_PREFIX}group $id\";; \
pid) kill -KILL \"$id\" 2>/dev/null && echo \"{SIGNALLED_PREFIX}pid $id\";; \
agent) kill -KILL \"$id\" 2>/dev/null && echo \"{SIGNALLED_PREFIX}agent $id\";; \
esac; \
done; \
exit 0"
    )
}

/// How many groups and pids the sweep actually signalled, from its own report.
/// A kill that found nothing left prints nothing, so this is the count of what
/// was still alive — the number worth logging.
pub(super) fn count_signalled_process_trees(stdout: &str) -> usize {
    stdout
        .lines()
        .filter(|line| line.trim_start().starts_with(SIGNALLED_PREFIX))
        .count()
}

#[cfg(test)]
mod tests {
    use super::{build_wsl_kill_agent_process_trees_command_line, count_signalled_process_trees};

    #[test]
    fn the_kill_plan_walks_descendants_and_never_group_kills_the_agents_own_group() {
        let command = build_wsl_kill_agent_process_trees_command_line(&[4242]);

        assert!(command.contains("agents='4242'"));
        assert!(command.contains("ps -e -o pid=,ppid=,pgid="));
        // Descendant groups are killed as groups; the agent's own group never is.
        assert!(command.contains("kill -KILL -- \"-$id\""));
        assert!(
            command.contains("if (dg > 1 && dg != own) { groups[dg] = 1 } else { lone[d] = 1 }")
        );
        // The agent goes last, as a lone pid.
        assert!(command.contains("agent) kill -KILL \"$id\""));
        // A failing `ps` is an error, not an empty plan.
        assert!(command.contains("set -uo pipefail"));
        assert!(command.contains("|| exit 3"));
    }

    /// The whole awk program travels inside one single-quoted shell word, so a
    /// single quote anywhere in it would end that word and corrupt the script.
    #[test]
    fn the_kill_plan_script_carries_no_quote_that_would_break_the_shell_word() {
        let command = build_wsl_kill_agent_process_trees_command_line(&[7, 9]);
        let awk_start = command
            .find("awk -v agents=\"$agents\" '")
            .expect("awk word");
        let program = &command[awk_start + "awk -v agents=\"$agents\" '".len()..];
        let program_end = program.find('\'').expect("closing quote");
        assert!(
            program[..program_end].contains("BEGIN { wanted = split(agents, want, \" \") }"),
            "the awk program must survive intact inside its single-quoted word"
        );
        assert_eq!(
            &program[program_end..program_end + 2],
            "')",
            "the only quote inside the awk word must be the one that closes it"
        );
    }

    #[test]
    fn an_empty_agent_list_still_produces_a_runnable_script() {
        let command = build_wsl_kill_agent_process_trees_command_line(&[]);
        assert!(command.contains("agents=''"));
    }

    #[test]
    fn signalled_lines_are_counted_and_other_output_is_not() {
        let stdout = "signalled group 100\nsignalled pid 101\nsignalled agent 42\n";
        assert_eq!(count_signalled_process_trees(stdout), 3);
        assert_eq!(count_signalled_process_trees(""), 0);
        assert_eq!(count_signalled_process_trees("ps: some warning\n"), 0);
    }

    /// Prints the real script for the manual in-distro witness, so the thing
    /// exercised through `wsl.exe` is the builder itself and not a transcription.
    /// See `docs/commands/verify_wsl_agent_tree_kill.md`.
    #[test]
    #[ignore = "prints the in-distro kill script for the manual WSL witness"]
    fn print_wsl_kill_agent_process_trees_script() {
        let raw = std::env::var("INTERMEDIARY_TEST_AGENT_PIDS")
            .expect("set INTERMEDIARY_TEST_AGENT_PIDS to a space-separated pid list");
        let pids: Vec<u32> = raw
            .split_whitespace()
            .map(|value| value.parse::<u32>().expect("pid must be a u32"))
            .collect();
        println!(
            "----SCRIPT----\n{}\n----END----",
            build_wsl_kill_agent_process_trees_command_line(&pids)
        );
    }
}
