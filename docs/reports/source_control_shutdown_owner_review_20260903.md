# Intermediary Source Control — WSL Shutdown Owner Review (fourth adversarial review)

Updated on: 2026-09-03
Owners: JL · External reviewer
Depends on: ADR-007, ADR-008, ADR-009
Reviewed bundle: `dd6d8f2` (round 3 landed)

## Verdict

Almost closed. Six of the seven prior findings are genuinely closed or explicitly and coherently
adjudicated; the rejected private-commit transaction, trusted-hook policy, per-refresh content hashing,
restore window, and cross-volume support are not reopened — the source records them as product
decisions. Commit retraction is gone, composite snapshot identity is real, discard recovery is materially
safer, the WSL ledger is cleared correctly, and the Source Control ownership split is present.

One concrete P0 remains in the WSL emergency-shutdown route.

## P0 — Tauri can still kill the WSL process-tree owner before its 450-second drain runs

The ordinary route is correct: Tauri asks the host agent to shut down; the host agent drains the WSL
backend first; each agent gets one shared 450-second emergency envelope.

The failure occurs when the host shutdown is unconfirmed — the crash, hang, or transport-loss case this
hardening is supposed to own:

1. After stopping the host, Tauri always proceeds to `stop_process(ProcessKind::Wsl)`
   (`src-tauri/src/lib/agent/supervisor/lifecycle.rs`, `supervisor/managed_processes.rs`).
2. That route sends `SIGTERM` directly to the WSL `im_agent` pid, waits only 750 ms, then sends
   `SIGKILL` to that pid (`supervisor/wsl_runtime.rs`, `agent/wsl_process_control.rs`,
   `agent/wsl_process_control_commands.rs`).
3. `im_agent`'s SIGTERM handler is designed to spend up to 450 seconds draining mutations; only after
   that does it call `terminate_git_process_trees` (`crates/im_agent/src/server/shutdown.rs`).
4. Each Git invocation and its hooks/helpers live in a separate Unix process group
   (`crates/im_bundle/src/git_capture/command_tree.rs`).

Killing only the agent pid after 750 ms does not kill those process groups; the registry that knew their
pgids dies with the agent. The outer fallback does not close this: app exit skips distro termination when
host finality is unknown, and even with known finality skips it while an interactive WSL session is open
(`supervisor/shutdown.rs`). There is also no EOF shutdown owner: the design doc says the WSL agent drains
on "SIGTERM/EOF", but websocket EOF only ends the connection handler.

### Concrete failure construction

A WSL commit is running a long hook. The host agent crashes before forwarding or completing shutdown.
The user closes Intermediary. Tauri observes unconfirmed host finality, sends `SIGTERM` to the WSL
`im_agent`, interrupts its newly started 450-second drain after 750 ms with `SIGKILL`, leaves the
Git/hook process group alive, and skips distro termination because host finality was unknown. The hook
can continue mutating the worktree or holding `.git/index.lock` after Intermediary has exited.

## Required correction

The WSL emergency owner must terminate the whole in-distro descendant boundary, not just `im_agent`:
send SIGTERM to `im_agent`; allow its full 450-second drain plus exit margin; if it fails to exit, have
the outer WSL control path terminate the agent's Git process groups or recursively terminate its
in-distro descendant tree before killing the agent; do not depend on whole-distro termination, because
preserving an interactive WSL session is an explicit product behaviour.

This is one narrow shutdown-owner defect, not another Source Control redesign.
