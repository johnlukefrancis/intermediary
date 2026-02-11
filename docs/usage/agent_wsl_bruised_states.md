# Agent/WSL Bruised States Runbook
Updated on: 2026-02-11
Owners: JL · Agents
Depends on: ADR-000, ADR-006, ADR-007, ADR-010, ADR-012

## Purpose

This runbook covers **degraded but functioning** runtime states where Intermediary should recover without reinstall/reset.

## 1) WSL backend offline, host still works

Observed behavior:
- Windows/host-root repos continue to watch, stage, and build normally.
- WSL-root actions return explicit WSL transport errors.
- UI may show `Agent offline` diagnostics and/or WSL transport error text while WSL transport is failing.

Why this is expected:
- Host and WSL backends are split by root authority (`host` vs `wsl`).
- WSL transport outages are isolated so host workflows keep running.

What to do:
- Wait for auto-reconnect if WSL is briefly unavailable.
- After backend recovery, run any WSL operation (for example refresh/stage/build on a WSL repo); the first successful WSL command emits `wslBackendStatus: online` and clears the offline banner even if the socket never disconnected.
- Use **Restart Agent** if WSL remains offline.

## 2) After sleep/wake, app rehydrates

Expected behavior after resume:
- Status bar may briefly show `Reconnecting (...)`.
- UI reconnects the WebSocket session.
- `clientHello` is replayed as needed and repo/bundle state rehydrates.
- Stale WSL transport errors clear after explicit `wslBackendStatus: online`, including the recovery signal emitted on the first successful WSL operation after transport recovery.

When to escalate:
- Reconnecting state does not clear after a reasonable window.
- WSL-only actions keep failing after reconnect appears healthy.

## 3) Bundle timeouts (bruised state)

Observed behavior:
- Bundle requests are timeout-bounded (notably 5 minutes for build requests).
- A timed-out build returns an error to UI, but does **not** replace/remove the previous successful bundle.

Why this is safe:
- Bundle finalize uses temp file + atomic rename.
- Older bundles are pruned only after successful finalize.

What to do:
- Retry build once backend is online/stable.
- If repeated timeout persists, use **Restart Agent** and rebuild.

## 4) Mounted Windows paths in Linux/WSL runtime (warn-only)

Observed behavior:
- If a repo root resolves to `/mnt/<drive>/...` in a Linux/WSL runtime, Intermediary emits a watcher warning in the status bar.
- The watcher still starts, but change detection can be degraded on large or busy trees.

Why this is expected:
- Linux/WSL filesystem watch reliability is lower for mounted Windows paths than for native Linux paths.
- Intermediary keeps this mode available for flexibility, but warns explicitly instead of silently failing.

What to do:
- Prefer `Tauri: dev (Windows)` or `Tauri: dev (Windows, watch + sync)` for Windows-root repos.
- Keep Linux/WSL runtime usage for native Linux roots (for example `/home/<user>/...`).

## 5) Log locations

Default runtime logs are under the app local-data `logs` directory:

| Platform | Log directory | Files |
|---|---|---|
| Windows | `%LOCALAPPDATA%\\Intermediary\\logs` | `run_latest.txt`, `agent_latest.log` |
| macOS | `~/Library/Application Support/Intermediary/logs` | `run_latest.txt`, `agent_latest.log` |
| Linux | `~/.local/share/intermediary/logs` (or `$XDG_DATA_HOME/intermediary/logs`) | `run_latest.txt`, `agent_latest.log` |

Notes:
- `run_latest.txt` is app/supervisor-side logging.
- `agent_latest.log` is host/WSL agent JSONL logging.
- Supervised app launches diagnose agent early exits from bounded `agent_latest.log` tails; they do not depend on draining child stdout/stderr pipes.
- Dev workflows may override log directory (for example via `INTERMEDIARY_LOG_DIR`).

## 6) Restart Agent: what it resets

`Restart Agent` performs supervisor stop + start for managed host/WSL agent processes.

Resets:
- Active WebSocket session(s) and in-memory request pipelines.
- Managed agent child processes (host and, when required, WSL backend).
- Transport generation/state used for WSL offline/online transitions.

Does not reset:
- User config, repos, starred files, notes, or staged/bundle files on disk.
- Persisted recent-files history (it rehydrates from runtime + persisted state).

Use Restart Agent when:
- WSL transport remains offline.
- Reconnect loops persist after sleep/wake.
- Bundle requests repeatedly timeout in a way that does not self-recover.

## 7) Stale WSL agent holding backend port (auto-remediated)

Observed behavior:
- WSL backend port is listening, but websocket auth probe rejects (`wrong token` / stale backend).
- This can happen when a previous WSL `im_agent` process survives while the supervisor only has a stale `wsl.exe` wrapper handle.

Automatic remediation now:
- Supervisor tracks the absolute WSL binary path (`<agent_dir_in_wsl>/im_agent`) for the launched backend.
- On **Stop Agent**, **Restart Agent**, and WSL auth-mismatch readiness failures, supervisor runs in-distro termination:
  - `pgrep -f '<agent_bin_wsl>' | xargs -r kill -TERM`
  - waits a short grace window
  - escalates to `kill -KILL` for the same matched process only if needed
- If the port is still occupied after remediation, supervisor performs one bounded retry with backoff, then returns a clear stale-port error.

Safety constraints:
- Termination is path-targeted to the configured agent binary and distro; it does not use global `pkill im_agent`.
- Host backend remains independent; this remediation only affects the WSL backend process match.
