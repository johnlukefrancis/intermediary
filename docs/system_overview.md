# Intermediary System Overview

Updated on: 2026-02-05
Owners: JL · Agents
Depends on: ADR-000, ADR-007, ADR-010

## Goal

Reduce friction when sharing files and context bundles between local repos (often in WSL) and browser-based LLM interfaces like ChatGPT. Intermediary is a single-window "handoff console" that surfaces recently changed files and generates standardized zip bundles for drag-and-drop sharing.

## Architecture

Intermediary uses a **two-component architecture**:

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
│  │              Staging Directory                      │    │
│  │     %LOCALAPPDATA%\Intermediary\staging\            │    │
│  └─────────────────────────────────────────────────────┘    │
└────────────────────────┬────────────────────────────────────┘
                         │
           ══════════════╪══════════════  WSL Boundary
                         │
┌────────────────────────┴────────────────────────────────────┐
│                      WSL (Linux)                            │
│  ┌─────────────────────────────────────────────────────┐    │
│  │                  WSL Agent (Daemon)                 │    │
│  │                                                     │    │
│  │  • Watches repos via inotify                        │    │
│  │  • Provides "recent changes" feed                   │    │
│  │  • Builds zip bundles to staging                    │    │
│  │  • Stages individual files on request               │    │
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
- **Purpose:** Ensure the WSL agent is installed and running when the app is open
- **Key features:**
  - Installs bundled Rust agent binary (`im_agent`) into `%LOCALAPPDATA%\Intermediary\agent`
  - Launches the agent via `wsl.exe` with explicit env configuration
  - Auto-start toggle with optional distro override
  - Restart command and diagnostics surfaced in the UI
  - Stops the agent on app exit to avoid orphaned WSL processes

### WSL Agent

- **Stack:** Rust (Tokio + notify) with the `im_bundle` library for bundle creation
- **Purpose:** File watching and bundle generation inside WSL
- **Key features:**
  - inotify-based file watching via notify (reliable for Linux FS)
  - Recent changes feed with 250ms debouncing and persisted history under `staging/state/recent_files/<repoId>.json`
  - Bundle building with manifest injection via `im_bundle` (single latest bundle per preset; older bundles deleted)
  - Atomic file staging to Windows-accessible paths
  - Auto-stage on change (configurable)

### IPC Protocol

Communication via WebSocket on `127.0.0.1:<port>`, with request/response envelopes and event envelopes:
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

Windows filesystem watchers (`ReadDirectoryChangesW`) are unreliable for WSL UNC paths (`\\wsl$\...`). The WSL agent uses native Linux inotify for reliable file watching, then communicates changes to the Windows UI.

**v0 constraint:** Repos are persisted as path-native roots (`{ kind: "wsl" | "windows", path }`). The current WSL agent watches only `wsl` roots; `windows` roots stay Windows-native and are reserved for the upcoming Windows watcher backend.

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

1. **File Change → UI Update:** Repo file changes → inotify → WSL agent → WebSocket → UI updates recent list
2. **Drag-out:** User drags row → UI requests staging → Agent copies to staging → UI initiates OS drag with Windows path
3. **Bundle Build:** User clicks Build → UI requests bundle → Agent deletes prior bundle for preset → Agent zips + writes manifest → Agent stages to Windows → UI shows the latest bundle

## Related docs

- [docs/prd.md](prd.md) — Full product requirements
