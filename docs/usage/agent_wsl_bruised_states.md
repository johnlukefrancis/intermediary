# Agent/WSL Bruised States Runbook
Updated on: 2026-07-10
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
- `clientHello` is replayed once for the active WSL connection generation and repo/bundle state rehydrates.
- Recursive watcher registration runs concurrently off the async runtime workers; the host uses the WSL client's single bounded request lifecycle rather than abandoning bootstrap under a shorter wrapper timeout.
- Stale WSL transport errors clear after explicit `wslBackendStatus: online`, including the recovery signal emitted on the first successful WSL operation after transport recovery.

When to escalate:
- Reconnecting state does not clear after a reasonable window.
- WSL-only actions keep failing after reconnect appears healthy.

## 3) Bundle timeouts (bruised state)

Observed behavior:
- Bundle requests are timeout-bounded (notably 5 minutes for build requests).
- A timed-out build returns an error to UI, but does **not** replace/remove the previous successful bundle.
- Timeout and disconnect cancellation use the same cooperative build token as the Cancel button; the blocking worker retains the repo/preset build lock until temporary output cleanup completes.
- While a bundle is building, the build button becomes **Cancel**. Cancellation is quiet in the UI: the in-progress state clears without recording a persistent build error.

Why this is safe:
- Bundle finalize uses temp file + atomic rename.
- Older bundles are pruned only after successful finalize.
- Cancellation is cooperative and build-id scoped. The backend removes only the matching in-progress temp zip and does not remove the previous successful bundle.
- A replacement build for the same repo/preset cannot start until the cancelled worker has finished cleanup and released its build lock.

What to do:
- Cancel a long build if the selection was wrong or the backend is unstable, then retry once backend is online/stable.
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
| Windows | `%LOCALAPPDATA%\\com.johnf.intermediary\\logs` | `run_latest.txt`, `agent_latest.log` |
| macOS | `~/Library/Application Support/Intermediary/logs` | `run_latest.txt`, `agent_latest.log` |
| Linux | `~/.local/share/intermediary/logs` (or `$XDG_DATA_HOME/intermediary/logs`) | `run_latest.txt`, `agent_latest.log` |

Notes:
- `run_latest.txt` is app/supervisor-side logging.
- `agent_latest.log` is host/WSL agent JSONL logging.
- WSL bootstrap completion/failure is anchored by `WSL backend clientHello applied` / `WSL backend clientHello failed`, with `durationMs` and `generation` fields.
- Supervised app launches diagnose agent early exits from bounded `agent_latest.log` tails; they do not depend on draining child stdout/stderr pipes.
- Dev workflows may override log directory (for example via `INTERMEDIARY_LOG_DIR`).

## 6) Restart Agent: what it resets

`Restart Agent` performs a **forced** supervisor stop + start for managed host/WSL agent processes. It bypasses the "already running" short-circuit, reclaims the WSL backend by port (even a wedged or token-mismatched backend this session did not spawn), and always respawns. It no longer silently no-ops on a healthy-looking-but-wedged backend, so it recovers the app from the wedged state without manually killing the port.

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
- It can also happen when another Intermediary install/dev task owns the same backend port with a different app-auth token.

Automatic remediation now:
- Supervisor tracks the absolute WSL binary path (`<agent_dir_in_wsl>/im_agent`) for the launched backend, plus a durable `last_wsl_backend` handle (distro + port) recorded on every ensure pass so reclamation works even for an adopted/reconnected agent it did not spawn.
- Startup validates the app-local installed agent bundle against the packaged resource bundle before reusing it, even when a backend is already listening.
- Reclaims by **port listener**: finds the PID(s) that own the reserved WSL backend port (`ss -H -ltnp "sport = :<port>"` inside the distro) and confirms each is our own `im_agent` by `/proc/<pid>/comm == im_agent`, executable basename, or an `INTERMEDIARY_WSL_WS_TOKEN` in its environment. This recognizes an Intermediary backend even when it was launched from a different install path or with a different token/port, so `detect_wsl_backend_owner` classifies it as `SamePortIntermediary` (reclaimable) instead of `ExternalUnmanaged`. Confirmed PIDs are terminated `TERM` → `KILL` by PID. If `ss` is unavailable it yields nothing and falls back to the path/env detectors below.
- On **Stop Agent**, **Restart Agent**, and WSL auth-mismatch readiness failures, supervisor runs in-distro termination:
  - lists only processes whose command line contains the exact configured agent binary path
  - accepts `/proc/<pid>/exe` matches and command-line fallback matches for the same configured path
  - for auth-mismatch remediation in auto/managed modes, also accepts same-port Intermediary `im_agent` processes whose environment includes the matching `INTERMEDIARY_AGENT_PORT` and an `INTERMEDIARY_WSL_WS_TOKEN`
  - sends `TERM` to the matched process IDs
  - waits a short grace window
  - escalates to `KILL` for the same matched process IDs only if needed
- If the port is still occupied after remediation, supervisor performs one bounded retry with backoff, then returns a clear stale-port error.
- In `external` WSL backend mode, supervisor never terminates the listener; token mismatch remains an external-backend setup error.

Safety constraints:
- Port-listener and path/env matching only ever confirm an Intermediary `im_agent`; a foreign (non-Intermediary) listener is never matched and never terminated, and it does not use global `pkill im_agent`. The hard "occupied by an external process … will not be terminated in `mode=auto`" refusal now fires only for a genuinely foreign listener.
- Termination is path-targeted to the configured agent binary, or same-port/env-targeted to an Intermediary WSL agent in the selected distro.
- Host backend remains independent; this remediation only affects the WSL backend process match.

## 8) Exit teardown: freeing WSL RAM

Observed behavior:
- When you close the app, the supervisor stops the managed agents (reliably, by port), then frees the WSL VM's RAM only when the distro is otherwise idle.
- "Idle" means no interactive `pts/*` session is open; console gettys (`hvc0`/`tty1`) and headless services are ignored.
- When idle, the supervisor runs `wsl --terminate <distro>` (targeted to the one distro, never `wsl --shutdown`), releasing the 4–6 GB the WSL VM typically holds once you are done.

Why this is safe:
- If any interactive WSL shell or terminal tab is open, the distro is left running so your other WSL work is never disrupted.
- In `external` WSL backend mode nothing is terminated.

Notes:
- A Task-Manager force-kill of the app cannot run cleanup, so the distro/agent can be left behind; the next launch reclaims the orphaned agent by port (see §7) and, on the following clean exit, teardown resumes.
