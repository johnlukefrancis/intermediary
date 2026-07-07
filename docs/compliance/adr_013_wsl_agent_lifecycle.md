# ADR-013: WSL Agent Lifecycle, Ownership, and Shutdown

Status: Accepted
Date: 2026-07-07
Owners: JL · Coding agents
Scope: WSL backend supervision (src-tauri/src/lib/agent/**)

---

## Context

On Windows the app supervises a background `im_agent` running inside WSL, reached over a websocket
(host `:agentPort`, WSL backend `:agentPort + 1`). Historically the supervisor's authority over that
process was **launch-session-anchored**: it could reliably terminate only an agent it spawned itself
this session, matched by exact absolute binary path or by an environment signature
(`INTERMEDIARY_AGENT_PORT` + `INTERMEDIARY_WSL_WS_TOKEN`). It had no way to ask *which PID owns the
backend port right now*.

This produced three linked, user-visible failures with no in-app recovery:
- **Reinstall / dev-task wedge.** A surviving agent from a prior install (or a closed WSL dev task)
  held the port with a different token; it fell through the narrow detectors, was classified
  `ExternalUnmanaged`, and in the default `auto` mode was refused termination.
- **"Restart Agent" no-op.** `stop()` only killed the in-distro agent when it held a launch target it
  recorded this session; a forced restart short-circuited to `AlreadyRunning`.
- **WSL RAM leak on close.** Exit never reliably killed the agent and never freed the WSL VM, which
  lingered holding 4–6 GB.

The auth token compounded the wedge: `ws_auth.json` lived inside the `agent/` directory the installer
wipes, so a version-bump reinstall minted a new random token guaranteed to mismatch the survivor.

There was no compliance-grade contract for backend ownership, reclamation, or shutdown — only prose.

## Decision

1) **Kill authority is port-anchored, not session-anchored.**
   The supervisor MUST be able to identify the PID(s) bound to the reserved WSL backend port
   (via `ss` inside the distro) and reclaim them. Reclamation MUST confirm each PID is an
   Intermediary `im_agent` — by `/proc/<pid>/comm`, executable basename, or the presence of
   `INTERMEDIARY_WSL_WS_TOKEN` — before signalling it.

2) **Reclaim our own; never touch foreign processes.**
   Any confirmed Intermediary `im_agent` on the reserved port is `SamePortIntermediary`
   (reclaimable), regardless of launch path, token value, or port env. A listener that is NOT a
   confirmed Intermediary agent is `ExternalUnmanaged` and MUST NOT be terminated; it yields an
   actionable error. Termination is always TERM→KILL of explicit PIDs — never global `pkill`.

3) **Stop and Restart are reliable and idempotent.**
   `stop` and `Restart Agent` MUST reclaim the backend by port using a durable distro+port handle
   recorded on every ensure pass, so they work for an adopted/reconnected agent the session did not
   spawn. A **forced restart MUST always tear down and respawn** — `force` bypasses any
   already-running short-circuit. Restart Agent is the single user-facing recovery for a wedged
   backend.

4) **Exit teardown is conditional and non-destructive.**
   On exit the supervisor MUST stop the agent by port, then terminate the distro
   (`wsl --terminate <distro>`, never `wsl --shutdown`) **only when the distro is otherwise idle** —
   no process is attached to a `pts/*` pseudo-terminal (no interactive WSL shell/tab open). Any open
   interactive session means the distro is left running. External-mode backends are never torn down.
   A Task-Manager force-kill cannot run cleanup; the next launch reclaims the orphan by port (rule 1).

5) **Auth token survives reinstalls.**
   `ws_auth.json` MUST live outside the installer-wiped `agent/` directory (directly under app-local
   data), with a one-time migration from the legacy location. Reinstalls reuse the token so a
   surviving backend authenticates rather than wedging.

6) **`external` mode is user-managed.**
   In `INTERMEDIARY_WSL_BACKEND_MODE=external` the supervisor never terminates the listener and never
   records a durable backend handle; token mismatch remains an external-backend setup error.

7) **WSL control scripts execute over stdin, without a login shell.**
   Detection, reclamation, signalling, and idle-probe scripts MUST be fed to `bash --noprofile --norc
   -s` over **stdin**, never embedded as a `wsl.exe` argument. `wsl.exe`'s Windows→WSL argument
   marshalling mangles embedded newlines, nested quotes, and `$()`; and a login shell runs the user's
   profile, which under the terminal-less `wsl.exe` launch emits terminal-size errors and exposes a
   `$PATH` containing Windows directories with spaces and parentheses. Both silently corrupt command
   output, which previously defeated every detector. (Agent *launch* is exempt only because its
   command is a fixed, absolute-path string with no such constructs.)

## Consequences

- Reinstalls and closed dev tasks self-heal: the wedge is reclaimed on startup, and (via the relocated
  token) usually avoided entirely by a clean reconnect.
- "Restart Agent" is a real kill+respawn that recovers a wedged backend without manual port-killing.
- Closing the app frees WSL RAM when the user is done, without disrupting other WSL shells or builds.
- Reclamation depends on `ss` (iproute2) inside the distro; when absent it degrades to the prior
  path/env detectors (no regression). The idle heuristic keys on interactive `pts/*` sessions, which
  matches the user's "no WSL tabs open" intent and ignores console gettys and headless services.
- Constrained by ADR-008 (typed errors, no panics across command boundaries) and ADR-009 (blocking
  WSL calls run on `spawn_blocking`, bounded, off the UI thread).
