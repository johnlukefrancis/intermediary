# Known Issues — Intermediary

Updated on: 2026-09-03
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

- 2026-07-15: A Windows host-native `wb-lab` context bundle selected 1,104 changed paths but
  emitted a zero-byte patch with four Git command failures. The selected pathspec arguments were
  within Intermediary's 4,096-path/256 KiB product bound but exceeded the smaller Windows process
  command-line ceiling before Git could start. The source fix divides selected paths into
  deterministic 24 KiB process batches while keeping rename pairs atomic; a Windows build and
  regenerated real-repo bundle witness are still required before moving this issue to Resolved.

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

## Source Control — accepted boundaries and decisions

Recorded from the P2 findings of `docs/reports/source_control_adversarial_review_20260903.md` and
`docs/reports/source_control_hardening_review_20260903.md`. These are accepted end states, not open
defects; kept here so they are not mistaken for oversights. The rejected findings of the third review
(private commit transaction, per-status content digests, the `git restore` TOCTOU window, cross-volume
linked worktrees) are recorded as accepted boundaries in `docs/design/source_control_design.md`.

- 2026-09-03: `app/src/components/source_control/` stays at its 10 sibling modules. ADR-000's split
  threshold exists to break up folders that hold more than one concern; this one is a single concern —
  the Source Control column and the rows, commit box, notices, warnings, and copy it is made of — and
  every sibling is under the LOC cap. Splitting it would invent buckets rather than owners. The Rust
  `crates/im_agent/src/source_control/` folder, which did hold several concerns, was split by owner
  (`status/`, `commit/`, `discard/`, `actions/`, `diff/`, `locks/`, `runner/`) instead.
- 2026-09-03: Host in-process source-control reads (status/diff for host-rooted repos) are bounded by
  their Git timeout only and are not cancellable — the host agent has no cancellation path to serve one,
  so no UI cancel control is offered for a host read. WSL-routed reads remain cancellable.
- 2026-09-03: A forwarded WSL mutation that hits the host's timeout stays in the outstanding-mutation
  ledger for the rest of the host process: the action is cancelled passively, so the WSL agent suppresses
  its late answer and the host never learns the outcome. Consequence: a later shutdown while the WSL
  backend is offline waits the full emergency bound instead of treating the backend as drained. Timeouts
  sit at 280–420 s, so this is rare; the fix (let a cancelled mutation still answer) is outside the ledger.
- 2026-09-03: MERGE CONFLICTS has no section-wide stage/unstage. This is intentional, not a gap: conflicts
  are resolved per row so a bulk action can never mark a conflict-marker file resolved by accident.
- 2026-09-03: Paths Git reports that are not valid UTF-8 cannot cross the protocol, so they are counted in
  `omitted.unrepresentablePath` and never listed. Because a section action enumerates only the paths its
  section listed, STAGE ALL does not reach them either; they can only be staged from a terminal.
- 2026-09-03: The WSL emergency stop's descendant sweep does not reach a hook that started its own
  session. It walks the agent's descendants from one `ps -e -o pid=,ppid=,pgid=` snapshot and kills their
  process groups; a hook that called `setsid` has by definition left the agent's tree, so nothing above
  it can claim it by ownership and reaching it would mean killing by heuristic. Accepted: `setsid` in a
  hook is a deliberate detachment. The agent's *own* process group is likewise never group-killed —
  `wsl.exe` puts unrelated processes in it — so same-group descendants are killed one pid at a time.
- 2026-09-03: A WSL agent the app adopted rather than spawned has no supervisor stdin pipe, so losing the
  Tauri process does not by itself ask it to drain (`stdin_pipe=none` in the adoption log). It is still
  stopped by the `shutdown` command and by the emergency route on the next stop/exit, and the next launch
  reclaims it by port.
- 2026-09-03: Finality is chosen over speed at exit, and the two stop waits compound. `stop()` runs the
  host graceful stop first (up to `HOST_STOP_WAIT_BOUND`, 480 s) and only then the WSL emergency stop,
  whose drain wait is that same 480 s constant — so an app exit where *both* agents are wedged
  mid-mutation can take about 16 minutes before either emergency tree kill runs, and the startup
  stale-port remediation retries the same route (two attempts) when a wedged agent still holds the port.
  That is the accepted cost of never killing into a drain that may still hold `.git/index.lock`: the
  bound is the emergency, not the plan. Ordinary exits are unaffected — a drained agent has already
  exited, so the host wait ends on its ack and the WSL route's first probe returns `NoMatch` at once.

---

## Resolved (recent)

- 2026-09-03: The WSL agent's process tree had no owner once the supervisor killed the agent. The
  emergency route TERMed the agent and KILLed it 750 ms later, but the WSL agent's own SIGTERM drain runs
  up to 450 s and every Git command inside it owns a separate Unix process group — so the KILL orphaned
  those groups (hooks still mutating the worktree, still holding `.git/index.lock`) while the outer distro
  termination is deliberately skipped when host finality is unknown or an interactive WSL session is open.
  Closed by `src-tauri/src/lib/agent/wsl_agent_termination.rs`: TERM, then the agent's own drain waited out
  for the same 480 s envelope the host stop uses (one shared `HOST_STOP_WAIT_BOUND`), and only on expiry an
  in-distro sweep (`wsl_process_tree_commands.rs`) that kills every descendant process group, then the
  same-group descendants, then the agent — with the signalled count in the log. An already-exited agent
  still returns `NoMatch` on the first probe, so the ordinary stop/restart is unchanged.
- 2026-09-03: The design doc claimed the WSL agent drained on "SIGTERM/EOF" but no EOF owner existed —
  websocket EOF only ends a connection handler, correctly, because the host reconnects. Closed by
  `crates/im_agent/src/server/stdin_eof.rs` (a `spawn_blocking` reader on fd 0 when it is a pipe or socket,
  resolving the shutdown signal with `reason: "stdin-eof"` into the same drain SIGTERM takes) and by
  `spawn_wsl_agent_process` giving the WSL backend a piped stdin whose write end lives inside the recorded
  `Child`. A tty or `/dev/null` launch is never claimed, so terminal and script runs are unaffected.
- 2026-09-03: On Windows neither the Git process tree nor the host agent's own tree had an owner, so a
  descendant (hook, `ssh`, credential helper) that outlived its parent was detached rather than
  terminated. Closed by `crates/im_bundle/src/process_job.rs` with `git_capture/command_tree.rs` (every
  Git child runs inside a Job Object assigned immediately after the spawn; a mutation that cannot be
  given one is refused before it spawns) and by `src-tauri/src/lib/agent/process_control.rs` (the
  supervisor spawns the host agent into a supervisor-owned job and terminates it on the emergency kill
  path). Neither job carries a kill-on-close limit, so helpers that close their pipes outlive Git as on
  Unix; an adopted agent has no job and is stopped by binary identity. **Unverified on Windows:** both
  job objects type-check and cross-compile for `x86_64-pc-windows-msvc` but neither has been exercised
  at runtime against a real `git.exe` or a real hung agent.
- 2026-08-17: Windows installer tasks could sync current WSL source to the default D: mirror but
  build a separately configured C: mirror, producing a successful installer from stale source.
  Fixed by exporting the configured Windows mirror through `WSLENV` with path translation before
  every WSL sync/watch process, so sync and build share one authoritative mirror.
- 2026-08-17: Windows-root file-location actions (Auto Files, the bundle selector,
  workspace titles, and bundle rows) could fall back to Explorer's default location because
  repo-relative slash paths remained mixed with the Windows root and Explorer interpreted those
  forward slashes as command switches. Fixed by resolving host-repo files with native Windows
  separators, centralizing Explorer argument construction, and passing Explorer's `/select,`
  switch and exact target as one argument for every file reveal. The folder-only `/e,` switch is
  not part of the file-selection form.
- 2026-07-15: Legitimate source directories named `target`, including
  `crates/wb_render_wgpu/src/target`, could not stay selected because topology refresh re-applied the
  basename default and the bundle scanner independently treated it as an unconditional global
  exclude. Fixed by persisting exact positive `includedSubdirs` selections and giving them
  precedence over directory-name excludes in the shared scanner/Git predicate while leaving other
  generated `target` directories excluded.
- 2026-07-15: The tab-bar folder button could launch Explorer at its default location for
  host-native Windows repo roots. Fixed by passing Explorer's folder-view switch and exact target as
  one argument; WSL UNC targets use the same tested argument boundary.
- 2026-07-12: The Windows splashscreen could remain indefinitely until clicked, survive after the
  main window appeared, and keep the process alive after the main window closed. Frontend readiness
  could retire the splash before Tauri emitted `RunEvent::Ready`, after which the runtime callback
  could show that same splash again. Fixed by making one backend startup state machine serialize
  both event orders, activate the CSS-gated main WebView beneath the splash when runtime readiness
  wins, and retire the splash when either startup completes or the main window is destroyed.
- 2026-07-10: The status bar could repeatedly show `Timed out waiting for WSL backend clientHello response` while the UI remained connected and the WSL WebSocket stayed on the same live generation. WSL bootstrap synchronously registered and reset each repo's recursive watcher in sequence, while the host wrapped the WSL client's bounded request in a second, shorter timeout; that wrapper could drop the caller without sending the client's cancellation, leave backend work running, and attribute a stale completion to an incorrectly sampled generation. Fixed by moving native watcher registration/unregistration to blocking workers, starting/resetting repo watchers concurrently, using the WSL client as the sole timeout owner, carrying the serving connection generation through every success path, and making request-id cancellation cooperative. Bundle workers now retain their build lock until blocking cleanup finishes, while cancelled staging removes its temporary copy before completing. The four-repo production-path probe improved from 4.5–5.2 seconds to about 2.2 seconds.
- 2026-07-07: WSL agent detection and termination were silently corrupted when scripts were marshalled through `wsl.exe -- bash -lc "<script>"`: the Windows→WSL argument boundary mangled embedded newlines/quotes/`$()` (observed `syntax error near '<n>'`), and the login profile injected terminal-size errors plus a `$PATH` full of `Program Files (x86)` landmines. So port reclamation and stale-agent detection returned nothing and a wedged agent stayed branded `external` — the reclamation fixes below never actually ran on Windows. Fixed by feeding every WSL control/detection script over **stdin** to `bash --noprofile --norc -s`, so the script never crosses wsl.exe's argument parser and no login profile runs. Verified end-to-end through real `wsl.exe` from Rust.
- 2026-07-07: Reinstalling the app (or launching then closing the WSL dev task) could permanently wedge the WSL backend — a surviving `im_agent` held port 3142 with a mismatched token and was branded an `external` process the supervisor refused to terminate in mode=auto, with no in-app recovery. Fixed by making kill authority port-anchored: the supervisor now finds the port owner via `ss`, confirms it is an Intermediary `im_agent` (comm/exe/token-env), and reclaims it (TERM→KILL) — so any of our own stale/mismatched agents on the reserved port are reclaimed while foreign listeners are still left alone. Also relocated `ws_auth.json` out of the installer-wiped `agent/` dir (with migration) so reinstalls reuse the token and reconnect cleanly.
- 2026-07-07: "Restart Agent" could silently no-op — `stop()` only killed the in-distro agent when it held a launch target it recorded this session, and a forced restart still short-circuited to `AlreadyRunning`. Fixed by reclaiming the backend by port on stop/restart (durable distro+port handle) and honoring `force` so a restart always tears down and respawns, recovering the app even from a wedged state.
- 2026-07-07: Closing the app left the WSL `im_agent` running, so the WSL VM lingered holding 4–6 GB of RAM. Fixed by reliably stopping the agent by port on exit and then terminating the distro (`wsl --terminate <distro>`) only when it is otherwise idle (no interactive `pts/*` session open); open WSL shells and external-mode backends are left untouched. A Task-Manager force-kill of the Tauri process no longer orphans the agent: killing the supervisor closes the stdin pipe it holds open, and the WSL agent's stdin-EOF reader resolves that into the same drain SIGTERM takes, so the agent finishes its mutation and exits on its own. What a force-kill still skips is the distro teardown — nothing runs the idle probe or `wsl --terminate`, so the VM keeps its RAM until WSL's own idle timeout. Reclaiming the agent by port on the next launch is therefore no longer the ordinary path; it remains the recovery for the two cases where no EOF drain finished: an agent that was mid-mutation and is still draining when the next launch starts, and an adopted agent that never had a supervisor pipe (recorded under *Source Control — accepted boundaries and decisions*).
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
