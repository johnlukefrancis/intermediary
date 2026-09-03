# Intermediary System Overview

Updated on: 2026-09-03
Owners: JL · Agents
Depends on: ADR-000, ADR-007, ADR-010

## Goal

Reduce friction when sharing trustworthy local repo context with browser-based LLM interfaces like ChatGPT. Intermediary is a single-window "handoff console" that surfaces recently changed files, stages drag-and-drop-safe copies, and generates standardized timestamped bundles so users can hand off either broad repo context or the latest incremental files without Explorer and `\\wsl$` friction.

Maintainer-validated runtime today is Windows 10/11. WSL2 is the recommended path for the full WSL-backed workflow, while host-native Windows repo workflows are also validated. The codebase includes host-native paths beyond that target, but macOS and Linux are not yet validated to the same standard.

## Architecture

Intermediary uses a **host-routed architecture**:

```
┌─────────────────────────────────────────────────────────────┐
│                  Host OS (Windows / macOS / Linux)          │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              Tauri App (Host UI)                    │    │
│  │  ┌─────────────────────┐  ┌─────────────────────┐  │    │
│  │  │     Auto Files      │  │    Zip Bundles      │  │    │
│  │  │      Panel          │  │      Column         │  │    │
│  │  └──────────┬──────────┘  └─────────┬───────────┘  │    │
│  │             │                       │              │    │
│  │             └───────────────────────┘              │    │
│  │                     │                               │    │
│  │              Drag-out to OS                         │    │
│  └─────────────────────┬───────────────────────────────┘    │
│                        │ WebSocket IPC                      │
│  ┌─────────────────────┴───────────────────────────────┐    │
│  │                Host Agent (Rust)                    │    │
│  │  • Single endpoint UI connects to                  │    │
│  │  • Routes per-repo commands by root kind           │    │
│  │  • Handles host-native repos locally               │    │
│  │  • Forwards WSL repos to internal WSL backend      │    │
│  └─────────────────────────────────────────────────────┘    │
└────────────────────────┬────────────────────────────────────┘
                         │
           ══════════════╪══════════════  WSL Boundary
                         │
┌────────────────────────┴────────────────────────────────────┐
│                      WSL (Linux)                            │
│  ┌─────────────────────────────────────────────────────┐    │
│  │               WSL Backend Agent (Daemon)            │    │
│  │                                                     │    │
│  │  • Watches WSL repos via inotify                    │    │
│  │  • Handles WSL repo stage/build/top-level/list      │    │
│  │  • Streams repo events back to host agent           │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐    │
│  │                   Watched Repos                     │    │
│  │     (User-configured WSL/Windows paths)             │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

## Components

### Host UI (Tauri)

- **Stack:** Tauri + React/TypeScript
- **Purpose:** Single-window "handoff console" with repo tabs
- **Key features:**
  - Two-window startup handshake: all command-visible Rust state is registered on the Tauri Builder
    before either configured WebView can load or invoke a command; user setup performs no window RPC;
    one runtime-owned state machine serializes `RunEvent::Ready` and frontend readiness in either
    order. When runtime readiness arrives first it applies persisted bounds and activates the
    CSS-gated main WebView beneath the static splashscreen; frontend readiness retires the splash.
    Destroying the main window also retires the splash so it cannot keep the process alive by itself
  - Resolves `run_latest.txt` and installs a bounded panic hook before Tauri construction; an
    unusable `INTERMEDIARY_LOG_DIR` emits a diagnostic and falls back to app-local storage before
    startup continues. Explicit pre-build/setup/build-complete stage markers preserve the payload,
    source location, process id, and reached lifecycle stage of pre-setup callback failures
  - Two-column layout per repo: Auto Files and a right rail that switches between Zip Bundles and Source Control (`[ ZIPS ] [ SOURCE n ]`, persisted globally as `uiState.activeRail`; handset mode exposes the same three sections)
  - Source Control column: branch/upstream status line with refresh, pull, and push; commit box (Ctrl+Enter); STAGED CHANGES / CHANGES / MERGE CHANGES sections with per-row and per-section stage/unstage, per-file discard behind a confirm, and a read-only diff kind in the shared workspace. Git runs in the agent that owns the repo root; the UI never mutates a repo directly
  - Responsive runtime mode switching between standard and handset layouts based on window geometry (hysteresis: `>=980px` standard, `<=860px` handset; maximized forces standard)
  - Global window-surface opacity control (0-100, default 100) for terminal-style transparency
  - Independent global substrate texture-intensity control (0-100, default 100)
  - Shared workspace replaces the Auto Files panel for supported UTF-8 file scratch buffers or supported image previews; scratch edits never write back to repo files, and Markdown-like text buffers render a live semantic editor layer
  - Auto Files exposes Auto/Latest/Active sort modes plus All/Documents/Code/Images icon filters; rows render last active time, update count, and one consolidated left-to-right activity telemetry column with a weighted waveform and top-left 24-hour pulse strip
  - Auto Files is scoped by the active Zip Bundles preset after repo topology is ready: files excluded by the visible bundle selection are hidden from the left picker until the active preset selection includes them again
  - File-row and opened text-file title right-click context menus with `Open File`, `Open Containing Folder`, and `Copy Relative Path`
  - File-row double-click opens supported text files through the agent-routed `readTextFile` command and common image files through `readImageFile`
  - Zip Bundles column includes a lazy file explorer: root files are visible, expanded directories fetch direct child files/subdirectories on demand, file icon clicks toggle bundle inclusion, and file-name context menus reuse the same OS file actions as Auto Files rows
  - Latest built bundle rows remain native drag surfaces and expose a right-side location/download button that reveals the generated ZIP in the host file manager
  - Image previews render from Blob URLs created from agent-provided bytes; raw filesystem paths and `file://` sources are not used in the webview
  - Native drag-out via `tauri-plugin-drag`
  - Dark mode, glassmorphic styling
  - “WSL agent offline” banner with port diagnostics when the agent is unreachable
  - Tabs are driven by configured repos (repoId + label), no project-specific UI

### Agent Supervisor (Host)

- **Stack:** Tauri (Rust)
- **Purpose:** Ensure the host agent is installed and running when the app is open
- **Key features:**
  - Installs bundled host-agent runtime into the app local data `agent` directory
  - Validates app-local agent version and binary bytes against packaged resources, then compares an
    authenticated handshake SHA-256 with the executable bytes actually running before adopting a
    listener; disk mismatches install the packaged bundle, process mismatches replace the host agent
    or reclaimable WSL backend, and `external` WSL mode remains untouched
  - Launches the host agent on `agentPort` (UI endpoint)
  - On Windows only: launches the WSL backend agent on `agentPort + 1` when any configured repo has `root.kind = "wsl"`
  - Auto-start toggle with optional distro override (Windows-only control)
  - Restart command and diagnostics surfaced in the UI
  - Spawns managed host/WSL agents with stdio logging disabled and null stdio streams to avoid undrained pipe-buffer stalls; early-exit diagnostics read bounded tails from `agent_latest.log`
  - Reconciles tracked host/WSL child processes before spawn/replace/stop, and stops tracked children on app exit to enforce a no-orphan-process boundary for supervisor-owned processes
  - Reclaims the reserved WSL backend port from any confirmed Intermediary `im_agent` — the owning PID(s) are found by port listener (via `ss` inside the distro) and verified by `/proc/<pid>/comm`, executable basename, or `INTERMEDIARY_WSL_WS_TOKEN` in the process environment — so a stale agent from a prior install or a closed dev task is reclaimed even when it was launched from a different path or token; a genuinely foreign (non-Intermediary) listener is never terminated and still errors in `mode=auto`. `external` mode remains user-managed
  - **Restart Agent** forces a real teardown: it bypasses the "already running" short-circuit and always reclaims the port and respawns the backend, even when the backend is healthy (no more silent no-op)
  - On app exit, stops the agents by port, then frees the WSL VM RAM only when the distro is otherwise idle — no interactive `pts/*` session is open (console gettys on `hvc0`/`tty1` and headless services are ignored) — by running `wsl --terminate <distro>` (targeted to the one distro, never `wsl --shutdown`); when any interactive WSL shell/tab is open the distro is left running, and in `external` mode nothing is terminated

### Host Agent

- **Stack:** Rust (Tokio + WebSocket)
- **Purpose:** Single UI-facing endpoint and per-repo backend router
- **Key features:**
  - Maintains repo backend map from path-native roots (`wsl` vs `host`)
  - Handles host-native roots locally for watch/refresh/stage/build/list/top-level
  - Maintains internal WebSocket client to WSL backend agent
  - Forwards WSL-targeted requests and relays backend events to the UI
  - Keeps retrying WSL backend transport with bounded reconnect delay and emits explicit online/offline transition events (`wslBackendStatus`) with a reconnect generation counter
  - Owns one bounded timeout per forwarded request; timed-out requests send a request-id-scoped cooperative cancellation to the WSL backend and let the operation retain its cleanup guards until work stops
  - Attributes every successful forwarded response to the connection generation that produced it, including `clientHello`, options, build, and build-cancellation paths, so an older response cannot clear a newer transport error
  - Emits explicit backend-availability errors without taking down Windows repos
  - Dispatches source-control commands without holding the runtime write lock: the backend is resolved under a short read lock, then host repos run Git in-process and WSL repos are forwarded, so a long push never freezes other repos or WSL forwards

### WSL Backend Agent

- **Stack:** Rust (Tokio + notify) with the `im_bundle` library for bundle creation
- **Purpose:** File watching and bundle generation for WSL roots
- **Key features:**
  - inotify-based file watching via notify (reliable for Linux FS)
  - Recursive native watcher registration/unregistration runs on blocking workers, with independent repo watchers started and reset concurrently during `clientHello` bootstrap
  - Emits `sourceControlChanged` (coalesced to at most one per 250ms with a trailing emit) for `.git` metadata writes and working-tree changes outside the repo's structural ignore globs; linked worktrees get a second watch on their real git dir
  - Source-control commands run Git through the shared `im_bundle::git` facade on blocking workers, serialized per repo for mutations, with reads cancellable and mutations bounded by timeout only
  - Recent changes index with 250ms debouncing, persisted history under `staging/state/recent_files/<repoId>.json`, and per-file activity metadata for Auto Files ranking
  - Bundle building via `im_bundle` with a v2 manifest, selection-bounded captured-HEAD Git status/patch evidence, host-safe batching for Windows-scale selected path sets, and generated handoff orientation (atomic finalize + prune old bundles only after finalize; the blocking worker owns the build lock through cancellation and cleanup)
  - Atomic file staging for WSL repo operations, with cooperative cancellation removing temporary copies before the request completes
  - Auto-stage on change (configurable)

### IPC Protocol

UI communication is via WebSocket on `127.0.0.1:<hostPort>` to the host agent, with request/response envelopes and event envelopes:
- The handshake requires an app-scoped query token loaded from app-local auth state (`ws://127.0.0.1:<hostPort>/?token=...`).
- Successful authenticated upgrades include the running executable's SHA-256 for supervisor-only
  lifecycle coherence checks; unauthenticated requests receive no runtime identity metadata.
- The app-scoped token lives in `ws_auth.json` directly under app-local data (`%LOCALAPPDATA%\com.johnf.intermediary\ws_auth.json`), outside the `agent/` subdirectory that the installer wipes on every version-bump reinstall; a one-time migration adopts any legacy `agent/ws_auth.json`, so reinstalls reuse the same token and a surviving backend still authenticates instead of wedging.
- Host-agent validates token for every upgrade and enforces origin allowlisting when an `Origin` header is present.
- Host→WSL backend forwarding uses a separate internal token not exposed to the UI.
- Request: `{ kind: "request", requestId, payload }`
- Host→WSL cancellation: `{ kind: "cancel", requestId }`; the backend signals only that connection's matching operation, suppresses its stale response, and retains the active request until cooperative cleanup completes
- Response: `{ kind: "response", requestId, status, payload|error }`
- Event: `{ kind: "event", eventId, payload }`

**Agent → UI events:**
- `fileChanged { repoId, path, kind, changeType, mtime, activity?, staged? }`, where `activity.history` carries 24-hour bucket data for Auto Files telemetry
- `snapshot { repoId, recent: FileEntry[] }`
- `repoTopologyChanged { repoId }` emitted when watcher events can invalidate top-level files, top-level directories, or bundle-selector subdirectory metadata up to repo depth 4
- `bundleBuilt { repoId, presetId, hostPath, aliasHostPath, bytes, fileCount, builtAtIso }`
- `sourceControlChanged { repoId }` emitted (coalesced) when Git metadata or the working tree changes in a way that can move `git status`
- `error { scope, message, details? }`
- `wslBackendStatus { status: "online" | "offline", generation }` emitted on WSL transport transitions; generation increments on each successful reconnect
- `hello` is defined in protocol types but not emitted in the current agent; handshake uses `clientHello` → `clientHelloResult`.

**UI → Agent commands (request/response):**
- `clientHello { config, stagingHostRoot, stagingWslRoot?, autoStageOnChange? } → clientHelloResult`
- `clientHello` may be sent on initial connect and reconnect; the agent treats it as idempotent and safe to re-run.
- `setOptions { autoStageOnChange? } → setOptionsResult`
- `watchRepo { repoId } → watchRepoResult`
- `refresh { repoId } → refreshResult`
- `stageFile { repoId, path } → stageFileResult`
- `readTextFile { repoId, path } → readTextFileResult`
- `readImageFile { repoId, path } → readImageFileResult`
- `listRepoDirectory { repoId, path } → listRepoDirectoryResult`; `path` is repo-relative and `""` lists the repo root, while non-root paths return direct child directories/files as repo-relative paths
- `buildBundle { repoId, presetId, buildId, selection } → buildBundleResult`
- `cancelBundleBuild { repoId, presetId, buildId } → cancelBundleBuildResult`; cancellation targets only the matching active build and leaves prior successful bundles intact.
- `getRepoTopLevel { repoId } → getRepoTopLevelResult`
- `listBundles { repoId, presetId } → listBundlesResult`
- `sourceControlStatus { repoId } → sourceControlStatusResult { repoId, status }`; whole-repository porcelain-v2 status projected onto the configured root (staged paths above the root and non-UTF-8 paths are counted in `status.omitted`, `status.committable` is Git's own answer, output over 8 MiB sets `status.truncated`)
- `sourceControlDiff { repoId, path, originalPath?, area: "index" | "worktree" } → sourceControlDiffResult` (2 MiB bound, `binary` and `truncated` flags)
- `sourceControlAction { repoId, action } → sourceControlActionResult { repoId, kind, status, commitSha? }`; `action.kind` is `stage` / `unstage` (scope `all` or `paths`), `discard` (paths), `commit` (message), `push`, `pull`. Mutations are serialized per repo, return the fresh status, and are never cancelled mid-command; timeouts are strictly nested per Git command < host→WSL request < UI request (status/diff 20/90/120 s, index actions 60/120/150 s, commit 120/240/300 s, push/pull 180/300/360 s)
- `getTrFleetStatus {} → getTrFleetStatusResult` (host-agent only; polls TR build ports 5601–5605 `__trdev/status` + `__trdev/doctor`)
- `trFleetAction { action, port, backend? } → trFleetActionResult` (host-agent only; `rebuild` / `restartWatch` with control header)

### Lifecycle recovery behavior

- If the WSL backend goes offline, Windows-root repos continue to function; WSL-targeted commands return explicit transport errors and status remains recoverable.
- On reconnect, host runtime replays cached WSL `clientHello` once per backend connection generation so watchers/state re-bootstrap without requiring manual full app reset. A successful replay is recorded against the generation carried by its response, not a generation sampled before the request.
- Forwarding uses the WSL client's canonical per-command timeout. There is no shorter wrapper timeout that can abandon the caller while the same backend request remains active; a canonical timeout emits request-id cancellation and the backend retains operation guards through cleanup before accepting conflicting work.
- WSL `clientHello` diagnostics log `WSL backend clientHello applied` or `WSL backend clientHello failed` with `durationMs` and `generation`, without logging the configuration payload.
- On OS resume (sleep/wake), the UI triggers reconnect + rehydrate flow; users may briefly see `Reconnecting (...)` and then normal status once handshake and hydration complete.

| Situation | Expected visible outcome |
|---|---|
| UI→host socket is connected while WSL `clientHello` is still applying | Status may say `Connected` because the host endpoint is live; WSL hydration waits for its generation-scoped bootstrap instead of issuing duplicate handshakes. |
| Multiple WSL repos require watcher startup | Watchers initialize concurrently off async runtime workers; one slow repo does not serialize every independent watcher registration. |
| A WSL request exceeds its canonical request budget | One WSL timeout is surfaced, the matching operation is cooperatively cancelled by request ID, temporary output is cleaned before its guard releases, and host-root workflows remain available. |

### Host OS File Actions

- File-row context-menu actions are executed through Tauri commands.
- Command inputs are `root` (`{ kind: "wsl" | "host", path }`) + `relativePath` (repo-relative slash path), not frontend-built absolute paths.
- The backend validates relative paths, resolves host-visible paths (including Windows WSL conversion), and launches native file-manager/open handlers per OS.
- `Open File` and `Open All Files` are text-editor first on host OSes:
  - Windows text files open in Notepad.
  - macOS text files open in TextEdit.
  - Non-text files (or text-editor launch failure) fall back to OS default app open behavior.

### Staging System

Staging roots are resolved by the Tauri backend (`get_app_paths`) from the app local data directory:
- Host root (all platforms): `<app_local_data>/staging`
- Optional WSL mirror root (Windows only): `/mnt/<drive>/.../Intermediary/staging`

Layout under the staging root:
- Files: `staging/files/<repoId>/...`
- Bundles: `staging/bundles/<repoId>/<presetId>/...`

### Config Persistence

User preferences are persisted to `<app_local_data>/config.json`:

| Platform | Default config location |
|----------|----------------------|
| Windows  | `%LOCALAPPDATA%\com.johnf.intermediary\config.json` |
| macOS    | `~/Library/Application Support/Intermediary/config.json` |
| Linux    | `~/.local/share/intermediary/config.json` (or `$XDG_DATA_HOME`) |

Contents:
- **App config:** Agent host/port, auto-stage global setting, repo definitions
- **Classifier config:** Global classification excludes (parallel to bundle excludes)
- **UI state:** Last active repo (by repoId) + last active worktree per group + persisted window opacity and texture intensity
- **Bundle selections:** Per-repo, per-preset root-file toggle, top-level directory selections, nested subdirectory exclusions, and explicit per-file exclusions

Config is loaded on app startup via Tauri command and saved with debounce (500ms) on changes. Atomic writes (temp file + rename) prevent corruption.
The Options menu includes a "Reset all settings" action that restores defaults, clears repos/preferences, and wipes staging bundles, recent-file caches, and local notes without deleting repository files.

Per-repo notes are stored outside config under `<app_local_data>/notes/`, keyed by a collision-safe repoId-derived filename. Removing a repo or group triggers best-effort note deletion for removed repoIds.

The websocket auth token is stored separately in `<app_local_data>/ws_auth.json` (for example `%LOCALAPPDATA%\com.johnf.intermediary\ws_auth.json`), deliberately outside the `agent/` directory the installer wipes on every version-bump reinstall. The token therefore survives reinstalls, so a surviving backend re-authenticates instead of wedging; a one-time migration adopts any legacy `agent/ws_auth.json`.

## File Classification

- Repo watchers classify files in this order:
  1) image extension classifier
  2) `docsGlobs`
  3) `codeGlobs`
  4) fallback extension classifier (generated broad-language list)
- Classification excludes are applied at watcher time to suppress noisy/generated files in Auto Files.
- Classification excludes affect watcher history, while the topology-ready active Zip Bundles selection is a UI visibility filter for Auto Files and the content contract for bundle builds.

## Why This Architecture?

Windows filesystem watchers (`ReadDirectoryChangesW`) are unreliable for WSL UNC paths (`\\wsl$\...`), while WSL inotify is not a safe watcher surface for Windows drive mounts (`/mnt/c/...`) at scale. Host routing keeps each repo on its native backend.

Repos are persisted as root-authority roots (`{ kind: "wsl" | "host", path }`). The host agent enforces this split at runtime so no host-native repo is watched from WSL.

## Directory Structure

```
intermediary/
├── app/                    # Frontend (React/TS)
│   └── src/
│       ├── components/     # UI components
│       ├── hooks/          # React hooks (useAgent, useConfig, etc.)
│       ├── lib/            # Agent client, messages
│       ├── shared/         # Protocol types, config schema
│       ├── styles/         # CSS modules
│       └── tabs/           # Per-repo tab components
├── src-tauri/              # Tauri backend (Rust)
│   └── src/lib/
│       ├── commands/       # Tauri commands (paths, config)
│       ├── config/         # Config persistence (types, io)
│       ├── obs/            # Observability (logging)
│       └── paths/          # Path resolution, WSL conversion
├── crates/                 # Rust workspace crates
│   ├── im_agent/           # WSL agent (Rust)
│   ├── im_host_agent/      # Host routing agent (Rust)
│   └── im_bundle/          # Bundle library + CLI (scan + zip + manifest)
│   └── src/
│       ├── bundles/        # Bundle building
│       ├── repos/          # File watching
│       ├── server/         # WebSocket server, router
│       ├── source_control/ # Git status, diff, and index/commit/remote actions
│       ├── staging/        # File staging, path bridge
│       └── util/           # Logger, errors, categorizer
├── docs/                   # Documentation
│   ├── commands/           # ADR-012 compliant command docs
│   ├── compliance/         # ADRs
│   └── inventory/          # File ledger
├── scripts/                # Build and utility scripts
└── logs/                   # Runtime logs (run_latest.txt, agent_latest.log)
```

## Key Workflows

1. **File Change → UI Update:** Repo file changes → backend watcher (Windows local or WSL) → host agent event bus → UI updates the Auto Files table from the unified recent list after applying the topology-ready active Zip Bundles selection; topology-changing directory events also refresh bundle explorer root metadata
2. **Drag-out:** User drags row → UI requests staging from host agent → request routed by repo root kind → staged Windows path returned → UI starts OS drag
3. **Bundle Build:** User edits root/directory/file selections in the Zip Bundles explorer → host agent routes by repo kind → the shared blocking writer captures HEAD/status, scans current files and Git paths through one selection predicate, reconciles selected Git-ignored ordinary files, batches selected tracked paths below host process-argument ceilings while keeping rename pairs atomic, writes ordinary files, verifies repeated patch/status/ignore classification and selected bytes, emits manifest/status/patch/handoff entries, then atomically finalizes → host agent forwards `bundleBuilt` event and response

## Related docs

- [docs/prd.md](prd.md) — Full product requirements
- [docs/architecture/bundle_format_architecture.md](architecture/bundle_format_architecture.md) — Bundle v2 and captured Git evidence contract
- [docs/architecture/source_control_architecture.md](architecture/source_control_architecture.md) — Source Control ownership, refresh signal, cancellation, and timeout contract
