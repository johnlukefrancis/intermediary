# Intermediary System Overview

Updated on: 2026-02-06
Owners: JL · Agents
Depends on: ADR-000, ADR-007, ADR-010

## Goal

Reduce friction when sharing files and context bundles between local repos (often in WSL) and browser-based LLM interfaces like ChatGPT. Intermediary is a single-window "handoff console" that surfaces recently changed files and generates standardized zip bundles for drag-and-drop sharing.

## Architecture

Intermediary uses a **host-routed architecture**:

```
┌─────────────────────────────────────────────────────────────┐
│                     Windows Host                            │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              Tauri App (Windows UI)                 │    │
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
│  │  • Handles Windows repos locally                   │    │
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

### Windows UI (Tauri)

- **Stack:** Tauri + React/TypeScript
- **Purpose:** Single-window "handoff console" with repo tabs
- **Key features:**
  - Three-column layout per repo: Docs, Code, Zip Bundles
  - Native drag-out via `tauri-plugin-drag`
  - Dark mode, glassmorphic styling
  - “WSL agent offline” banner with port diagnostics when the agent is unreachable
  - Tabs are driven by configured repos (repoId + label), no project-specific UI

### Agent Supervisor (Windows)

- **Stack:** Tauri (Rust)
- **Purpose:** Ensure agent processes are installed and running when the app is open
- **Key features:**
  - Installs bundled Rust binaries (`im_host_agent.exe` and `im_agent`) into `%LOCALAPPDATA%\Intermediary\agent`
  - Launches the Windows-native host agent on `agentPort` (UI endpoint)
  - Launches the WSL backend agent on `agentPort + 1` only when any configured repo has `root.kind = "wsl"`
  - Auto-start toggle with optional distro override
  - Restart command and diagnostics surfaced in the UI
  - Stops agent processes on app exit to avoid orphans

### Host Agent

- **Stack:** Rust (Tokio + WebSocket)
- **Purpose:** Single UI-facing endpoint and per-repo backend router
- **Key features:**
  - Maintains repo backend map from path-native roots (`wsl` vs `windows`)
  - Handles Windows roots locally for watch/refresh/stage/build/list/top-level
  - Maintains internal WebSocket client to WSL backend agent
  - Forwards WSL-targeted requests and relays backend events to the UI
  - Emits explicit backend-availability errors without taking down Windows repos

### WSL Backend Agent

- **Stack:** Rust (Tokio + notify) with the `im_bundle` library for bundle creation
- **Purpose:** File watching and bundle generation for WSL roots
- **Key features:**
  - inotify-based file watching via notify (reliable for Linux FS)
  - Recent changes feed with 250ms debouncing and persisted history under `staging/state/recent_files/<repoId>.json`
  - Bundle building with manifest injection via `im_bundle` (single latest bundle per preset; older bundles deleted)
  - Atomic file staging for WSL repo operations
  - Auto-stage on change (configurable)

### IPC Protocol

UI communication is via WebSocket on `127.0.0.1:<hostPort>` to the host agent, with request/response envelopes and event envelopes:
- Request: `{ kind: "request", requestId, payload }`
- Response: `{ kind: "response", requestId, status, payload|error }`
- Event: `{ kind: "event", eventId, payload }`

**Agent → UI events:**
- `fileChanged { repoId, path, kind, changeType, mtime, staged? }`
- `snapshot { repoId, recent: FileEntry[] }`
- `bundleBuilt { repoId, presetId, windowsPath, aliasWindowsPath, bytes, fileCount, builtAtIso }`
- `error { scope, message, details? }`
- `hello` is defined in protocol types but not emitted in the current agent; handshake uses `clientHello` → `clientHelloResult`.

**UI → Agent commands (request/response):**
- `clientHello { config, stagingWslRoot, stagingWinRoot, autoStageOnChange? } → clientHelloResult`
- `clientHello` may be sent on initial connect and reconnect; the agent treats it as idempotent and safe to re-run.
- `setOptions { autoStageOnChange? } → setOptionsResult`
- `watchRepo { repoId } → watchRepoResult`
- `refresh { repoId } → refreshResult`
- `stageFile { repoId, path } → stageFileResult`
- `buildBundle { repoId, presetId, selection } → buildBundleResult`
- `getRepoTopLevel { repoId } → getRepoTopLevelResult`
- `listBundles { repoId, presetId } → listBundlesResult`

### Staging System

Staging roots are resolved by the Tauri backend (`get_app_paths`) from the app local data directory:
- Windows root: `%LOCALAPPDATA%\Intermediary\staging`
- WSL root: `/mnt/<drive>/Users/<user>/AppData/Local/Intermediary/staging`

Layout under the staging root:
- Files: `staging/files/<repoId>/...`
- Bundles: `staging/bundles/<repoId>/<presetId>/...`

### Config Persistence

User preferences are persisted to `%LOCALAPPDATA%\Intermediary\config.json`:
- **App config:** Agent host/port, auto-stage global setting, repo definitions
- **UI state:** Last active repo (by repoId) + last active worktree per group
- **Bundle selections:** Per-repo, per-preset directory selections

Config is loaded on app startup via Tauri command and saved with debounce (500ms) on changes. Atomic writes (temp file + rename) prevent corruption.
The Options menu includes a "Reset all settings" action that restores defaults, clears repos/preferences, and wipes staging bundles plus recent-file caches without deleting repository files.

## Why This Architecture?

Windows filesystem watchers (`ReadDirectoryChangesW`) are unreliable for WSL UNC paths (`\\wsl$\...`), while WSL inotify is not a safe watcher surface for Windows drive mounts (`/mnt/c/...`) at scale. Host routing keeps each repo on its native backend.

Repos are persisted as path-native roots (`{ kind: "wsl" | "windows", path }`). The host agent enforces this split at runtime so no Windows repo is watched from WSL.

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
