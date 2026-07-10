# Known Issues — Intermediary

Updated on: 2026-07-10
Owners: JL · Agents
Depends on: ADR-000, ADR-007

---

## Ground rules (keep this file tiny)

- Log only what is observed; do not add theories or speculation.
- Categorize by disruption level: P0 (blocked), P1 (major), P2 (degraded), P3 (minor).
- Keep entries short and actionable.
- Move resolved issues to the Resolved section with date.

---

## P0 — Core workflow blocked

- 2026-07-10: The Windows app can intermittently abort during startup before either window becomes
  usable. Repeated LocalDumps show Rust fast-fail `0xC0000409` on the non-unwinding-panic path; the
  crash stack enters Rust's cannot-unwind abort through a WebView2 event callback and retains the
  `StartupWindowState` state key. Tauri creates configured WebViews before calling user setup, so the
  frontend could race `startup_ready` against setup-time state registration and panic inside the COM
  callback. The source fix registers every command-visible state on the Builder before WebView
  creation, keeps readiness lookup fallible, initializes bounded panic/stage logging before Tauri
  construction with a diagnostic app-local fallback when a configured log directory is unusable,
  and defers launch window RPC to a one-shot `RunEvent::Ready` transition. A rebuilt
  Windows repeated-launch witness is still required before moving this issue to Resolved.

---

## P1 — Major functionality broken

*None*

---

## P2 — Degraded but usable

- 2026-07-10: After one Windows reinstall and automatic launch, the app opened with the agent offline;
  **Restart Agent** did not recover that run, while fully restarting the app restored normal host/WSL
  connectivity. The failure was intermittent and did not reproduce on the subsequent launch, so the
  release retains the startup diagnostics and this remains an observed follow-up rather than a
  claimed resolved path.
- 2026-02-08: macOS release packaging can fail to launch `im_host_agent` if helper-binary signing/notarization is incomplete. App now enforces executable permissions at install time and reports high-signal spawn errors, but final notarization coverage still depends on release pipeline configuration.
- 2026-02-11: WSL bundle builds are bounded by timeout windows (5 minutes for build requests). Very large or contended builds can return timeout while preserving the previously successful bundle; retry is usually sufficient after backend recovers.
- 2026-02-11: Linux/WSL runtime watching on mounted Windows paths (`/mnt/<drive>/...`) can be degraded on large or busy trees. Intermediary now emits a watcher warning with runbook guidance, but this mode remains warn-only (not blocked).
- 2026-07-10: `agent_latest.log` is append-only and the installed runtime log was observed at about 780 MB, dominated by successful supervisor health-probe connection lifecycle entries. Long-running installs can accumulate unnecessary disk usage until logging gains bounded retention and probe-aware verbosity.

---

## P3 — Minor issues

- 2026-02-11: After sleep/wake, status can briefly show `Reconnecting (...)` while the client reconnects and rehydrates repo state. This is expected during recovery, but can feel noisy on frequent wake cycles.

---

## Resolved (recent)

- 2026-07-10: The status bar could repeatedly show `Timed out waiting for WSL backend clientHello response` while the UI remained connected and the WSL WebSocket stayed on the same live generation. WSL bootstrap synchronously registered and reset each repo's recursive watcher in sequence, while the host wrapped the WSL client's bounded request in a second, shorter timeout; that wrapper could drop the caller without sending the client's cancellation, leave backend work running, and attribute a stale completion to an incorrectly sampled generation. Fixed by moving native watcher registration/unregistration to blocking workers, starting/resetting repo watchers concurrently, using the WSL client as the sole timeout owner, carrying the serving connection generation through every success path, and making request-id cancellation cooperative. Bundle workers now retain their build lock until blocking cleanup finishes, while cancelled staging removes its temporary copy before completing. The four-repo production-path probe improved from 4.5–5.2 seconds to about 2.2 seconds.
- 2026-07-07: WSL agent detection and termination were silently corrupted when scripts were marshalled through `wsl.exe -- bash -lc "<script>"`: the Windows→WSL argument boundary mangled embedded newlines/quotes/`$()` (observed `syntax error near '<n>'`), and the login profile injected terminal-size errors plus a `$PATH` full of `Program Files (x86)` landmines. So port reclamation and stale-agent detection returned nothing and a wedged agent stayed branded `external` — the reclamation fixes below never actually ran on Windows. Fixed by feeding every WSL control/detection script over **stdin** to `bash --noprofile --norc -s`, so the script never crosses wsl.exe's argument parser and no login profile runs. Verified end-to-end through real `wsl.exe` from Rust.
- 2026-07-07: Reinstalling the app (or launching then closing the WSL dev task) could permanently wedge the WSL backend — a surviving `im_agent` held port 3142 with a mismatched token and was branded an `external` process the supervisor refused to terminate in mode=auto, with no in-app recovery. Fixed by making kill authority port-anchored: the supervisor now finds the port owner via `ss`, confirms it is an Intermediary `im_agent` (comm/exe/token-env), and reclaims it (TERM→KILL) — so any of our own stale/mismatched agents on the reserved port are reclaimed while foreign listeners are still left alone. Also relocated `ws_auth.json` out of the installer-wiped `agent/` dir (with migration) so reinstalls reuse the token and reconnect cleanly.
- 2026-07-07: "Restart Agent" could silently no-op — `stop()` only killed the in-distro agent when it held a launch target it recorded this session, and a forced restart still short-circuited to `AlreadyRunning`. Fixed by reclaiming the backend by port on stop/restart (durable distro+port handle) and honoring `force` so a restart always tears down and respawns, recovering the app even from a wedged state.
- 2026-07-07: Closing the app left the WSL `im_agent` running, so the WSL VM lingered holding 4–6 GB of RAM. Fixed by reliably stopping the agent by port on exit and then terminating the distro (`wsl --terminate <distro>`) only when it is otherwise idle (no interactive `pts/*` session open); open WSL shells and external-mode backends are left untouched. A Task-Manager force-kill still skips cleanup, but the next launch reclaims the orphaned agent by port.
- 2026-07-07: Background GPU usage from substrate and status animations when the window was unfocused. The motion governor only paused on hide/minimize, and its CSS gate only paused the substrate. Fixed by pausing on any focus loss (shared `isForegroundWindow()` foreground test) and gating all animation via a universal `[data-motion="paused"]` rule, so the whole window's animation halts (GPU → near-idle) when it is not foreground and resumes on refocus.
- 2026-05-23: Settings Restart Agent could no-op after `WSL backend port 3142 is occupied by an external process that rejected the current websocket token` because the auto-mode error path cleared the WSL launch target before returning. Fixed by preserving the configured launch target before surfacing the auto-mode refusal, allowing Restart Agent to terminate the exact app-local backend path when it is the stale listener.
- 2026-05-22: Opening files in the containing folder could fail for Windows-path files. Fixed by passing Explorer's reveal selection and target path as one `/select,<path>` argument.
- 2026-05-22: Removing a folder or subfolder from the app could stall after confirmation because tab-dropdown outside-click handling could unmount portal confirmations before the confirm click ran. Fixed by making modal portals an explicit dropdown exclusion.
- 2026-05-22: Newly created folders inside a watched repo, for example `Docs/Screenshots`, could be missing until app restart. Fixed by adding a watcher topology-change event and refreshing the repo top-level directory model on that event.
- 2026-05-22: Fresh Windows installs and VS Code Windows dev tasks could report `WSL backend port 3142 is occupied by an external process that rejected the current websocket token` when a stale Intermediary WSL `im_agent` from another install/dev identity survived with a different token. Fixed by validating app-local agent binaries against packaged resources before reuse, treating same-port Intermediary WSL agent processes as bounded auto-remediation candidates outside `external` mode, and making the WSL dev task bootstrap/read the same app-local auth identity used by the Windows Tauri task.
- 2026-05-21: Bundle global excludes re-applied recommended defaults after users removed them, so source/control directories named `Build` could be omitted without manifest evidence. Fixed by treating explicit `globalExcludes` as authoritative and recording `effectiveGlobalExcludes` in bundle manifests.
- 2026-02-11: Supervised host/WSL agent processes could stall when logger stdout/stderr writes filled undrained pipe buffers. Fixed by disabling per-entry stdio emission for app-managed spawns, launching managed agents with null stdio streams, and using bounded `agent_latest.log` tails for early-exit diagnostics.
- 2026-02-11: Screenshot/image files with common extensions (`.png`, `.jpg`, `.jpeg`, `.webp`, etc.) are now classified as images so they appear in Auto Files instead of being filtered as `other`.
- 2026-02-11: Bundle build no longer requires delete-before-write semantics. Finalization now uses temp-write + atomic rename, then post-finalize pruning, so failed builds keep the last good bundle intact.
- 2026-02-11: WSL sleep/wake and backend restarts could leave stale `WSL backend is not available` errors in the status bar and skip WSL re-bootstrap when `clientHello` payloads were unchanged. Fixed by generation-aware WSL `clientHello` replay, transition-only WSL transport error emission, and explicit `wslBackendStatus` online/offline events that clear stale WSL transport errors on recovery.
- 2026-02-09: Installer builds could intermittently show `NOT CONFIGURED: Staging not configured` while agent status appeared connected. Fixed by gating staging-dependent actions on successful `clientHello`, adding one-shot `clientHello` re-sync + retry on staging-not-configured errors, and isolating dev channel identity/default agent port from installer defaults.
- 2026-02-09: Windows installer startup could briefly show empty file panels and transient `WSL backend is not available` before WSL repos hydrated. Fixed by adding a supervisor startup gate before first agent connect for WSL-required sessions, plus bounded backoff retries for repo and bundle hydration on transient WSL transport failures.
- 2026-02-08: macOS parity hardening (prompt 1/2 path) introduced repo hydration regressions (tab switch could drop snapshots) and bundle completion stalls (build UI could remain stuck at final stage). Fixed by restoring eager watcher startup during `clientHello`, removing bundle refresh gating on `watchedRepoIds`, and tightening host-runtime routing/error handling for unsupported roots.
- 2026-02-06: Narrow code classifier coverage could miss language families (for example `*.cpp`) in Auto Files. Fixed by generated broad-language extension coverage, default-only codeGlobs migration, and a separate classification-excludes model in Options.
- 2026-02-06: Windows repos stored as `/mnt/<drive>/...` were watched from WSL and could hang or stall change tracking on large trees. Fixed by path-native repo roots plus host-agent routing (Windows roots watched locally; WSL backend only for WSL roots).
- 2026-02-03: Production CSP blocked WebSocket agent connections. CSP allowed `ws://localhost:3141` but frontend dialed `ws://127.0.0.1:3141`. Fixed by aligning CSP with actual loopback URL and removing WSL IP resolution path.
- 2026-02-03: Config persistence failed after frontend schema bumped to v12 while Rust still enforced v11. Fixed by aligning versions, adding v11 to v12 loopback host migration, and a cross-check guard.
