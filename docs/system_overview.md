# Intermediary System Overview

Updated on: 2026-02-11
Owners: JL · Agents
Depends on: ADR-000, ADR-007, ADR-010

## Goal

Reduce friction when sharing files and context bundles between local repos (often in WSL) and browser-based LLM interfaces like ChatGPT. Intermediary is a single-window "handoff console" that surfaces recently changed files and generates standardized zip bundles for drag-and-drop sharing.

## Architecture

Intermediary uses a **host-routed architecture**:

```
┌─────────────────────────────────────────────────────────────┐
│                  Host OS (Windows / macOS / Linux)          │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              Tauri App (Host UI)                    │    │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────────────────┐  │    │
│  │  │  Docs   │  │  Code   │  │    Zip Bundles      │  │    │
│  │  │ Column  │  │ Column  │  │      Column         │  │    │
│  │  └────┬────┘  └────┬────┘  └─────────┬───────────┘  │    │
│  │       │            │                 │              │    │
│  │       └────────────┴─────────────────┘              │    │
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
  - Two-window startup handshake: static splashscreen shown immediately, main window hidden until frontend signals readiness
  - Three-column layout per repo: Docs, Code, Zip Bundles
  - Docs panel includes a per-repo plain-text Notes view (saved under app-local data)
  - File-row right-click context menu with `Open File`, `Open Containing Folder`, `Copy Relative Path`, and `Favourite/Unfavourite`
  - Native drag-out via `tauri-plugin-drag`
  - Dark mode, glassmorphic styling
  - “WSL agent offline” banner with port diagnostics when the agent is unreachable
  - Tabs are driven by configured repos (repoId + label), no project-specific UI

### Agent Supervisor (Host)

- **Stack:** Tauri (Rust)
- **Purpose:** Ensure the host agent is installed and running when the app is open
- **Key features:**
  - Installs bundled host-agent runtime into the app local data `agent` directory
  - Launches the host agent on `agentPort` (UI endpoint)
  - On Windows only: launches the WSL backend agent on `agentPort + 1` when any configured repo has `root.kind = "wsl"`
  - Auto-start toggle with optional distro override (Windows-only control)
  - Restart command and diagnostics surfaced in the UI
  - Reconciles tracked host/WSL child processes before spawn/replace/stop, and stops tracked children on app exit to enforce a no-orphan-process boundary for supervisor-owned processes

### Host Agent

- **Stack:** Rust (Tokio + WebSocket)
- **Purpose:** Single UI-facing endpoint and per-repo backend router
- **Key features:**
  - Maintains repo backend map from path-native roots (`wsl` vs `host`)
  - Handles host-native roots locally for watch/refresh/stage/build/list/top-level
  - Maintains internal WebSocket client to WSL backend agent
  - Forwards WSL-targeted requests and relays backend events to the UI
  - Keeps retrying WSL backend transport with bounded reconnect delay and emits explicit online/offline transition events (`wslBackendStatus`) with a reconnect generation counter
  - Emits explicit backend-availability errors without taking down Windows repos

### WSL Backend Agent

- **Stack:** Rust (Tokio + notify) with the `im_bundle` library for bundle creation
- **Purpose:** File watching and bundle generation for WSL roots
- **Key features:**
  - inotify-based file watching via notify (reliable for Linux FS)
  - Recent changes feed with 250ms debouncing and persisted history under `staging/state/recent_files/<repoId>.json`
  - Bundle building with manifest injection via `im_bundle` (atomic finalize + prune old bundles only after finalize; last-good bundle remains on build failure)
  - Atomic file staging for WSL repo operations
  - Auto-stage on change (configurable)

### IPC Protocol

UI communication is via WebSocket on `127.0.0.1:<hostPort>` to the host agent, with request/response envelopes and event envelopes:
- The handshake requires an app-scoped query token loaded from app-local auth state (`ws://127.0.0.1:<hostPort>/?token=...`).
- Host-agent validates token for every upgrade and enforces origin allowlisting when an `Origin` header is present.
- Host→WSL backend forwarding uses a separate internal token not exposed to the UI.
- Request: `{ kind: "request", requestId, payload }`
- Response: `{ kind: "response", requestId, status, payload|error }`
- Event: `{ kind: "event", eventId, payload }`

**Agent → UI events:**
- `fileChanged { repoId, path, kind, changeType, mtime, staged? }`
- `snapshot { repoId, recent: FileEntry[] }`
- `bundleBuilt { repoId, presetId, hostPath, aliasHostPath, bytes, fileCount, builtAtIso }`
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
- `buildBundle { repoId, presetId, selection } → buildBundleResult`
- `getRepoTopLevel { repoId } → getRepoTopLevelResult`
- `listBundles { repoId, presetId } → listBundlesResult`

### Lifecycle recovery behavior

- If the WSL backend goes offline, Windows-root repos continue to function; WSL-targeted commands return explicit transport errors and status remains recoverable.
- On reconnect, host runtime replays cached WSL `clientHello` once per backend connection generation so watchers/state re-bootstrap without requiring manual full app reset.
- On OS resume (sleep/wake), the UI triggers reconnect + rehydrate flow; users may briefly see `Reconnecting (...)` and then normal status once handshake and hydration complete.

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
| Windows  | `%LOCALAPPDATA%\Intermediary\config.json` |
| macOS    | `~/Library/Application Support/Intermediary/config.json` |
| Linux    | `~/.local/share/intermediary/config.json` (or `$XDG_DATA_HOME`) |

Contents:
- **App config:** Agent host/port, auto-stage global setting, repo definitions
- **Classifier config:** Global classification excludes (parallel to bundle excludes)
- **UI state:** Last active repo (by repoId) + last active worktree per group
- **Bundle selections:** Per-repo, per-preset directory selections

Config is loaded on app startup via Tauri command and saved with debounce (500ms) on changes. Atomic writes (temp file + rename) prevent corruption.
The Options menu includes a "Reset all settings" action that restores defaults, clears repos/preferences, and wipes staging bundles, recent-file caches, and local notes without deleting repository files.

Per-repo notes are stored outside config under `<app_local_data>/notes/`, keyed by a collision-safe repoId-derived filename. Removing a repo or group triggers best-effort note deletion for removed repoIds.

## File Classification

- Repo watchers classify files in this order:
  1) `docsGlobs`
  2) `codeGlobs`
  3) fallback extension classifier (generated broad-language list)
- Classification excludes are applied at watcher time to suppress noisy/generated files in Docs/Code panes.
- Bundle excludes remain separate and affect only zip build contents.

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

1. **File Change → UI Update:** Repo file changes → backend watcher (Windows local or WSL) → host agent event bus → UI updates recent list
2. **Drag-out:** User drags row → UI requests staging from host agent → request routed by repo root kind → staged Windows path returned → UI starts OS drag
3. **Bundle Build:** User clicks Build → host agent routes by repo kind → backend builds bundle/stages output → host agent forwards `bundleBuilt` event and response

## Related docs

- [docs/prd.md](prd.md) — Full product requirements
