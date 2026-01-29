# Intermediary System Overview

Updated on: 2026-01-29
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
│  │     /home/user/code/repo1                           │    │
│  │     /home/user/code/repo2                           │    │
│  │     ...                                             │    │
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

### WSL Agent

- **Stack:** Rust or Node.js (TBD based on spike results)
- **Purpose:** File watching and bundle generation inside WSL
- **Key features:**
  - inotify-based file watching (reliable for Linux FS)
  - Recent changes feed with debouncing
  - Bundle building with manifest injection
  - File staging to Windows-accessible paths

### IPC Protocol

Communication via WebSocket on `127.0.0.1:<port>`:

**Agent → UI:**
- `hello { agentVersion, distro, reposDetected }`
- `fileChanged { repoId, path, kind, mtime }`
- `snapshot { repoId, recent: FileEntry[] }`
- `bundleBuilt { repoId, presetId, windowsPath, size, mtime, gitShort }`
- `error { scope, message, details }`

**UI → Agent:**
- `watchRepo { repoId }`
- `refresh { repoId }`
- `stageFile { repoId, path } → { windowsPath }`
- `buildBundle { repoId, presetId } → { windowsPath }`

### Staging System

All draggable files originate from a staging directory on Windows:
- Location: `%LOCALAPPDATA%\Intermediary\staging\<repoId>\...`
- WSL agent writes to `/mnt/c/Users/<user>/AppData/Local/Intermediary/staging/...`
- UI references via Windows path `C:\Users\<user>\AppData\Local\Intermediary\staging\...`

## Why This Architecture?

Windows filesystem watchers (`ReadDirectoryChangesW`) are unreliable for WSL UNC paths (`\\wsl$\...`). The WSL agent uses native Linux inotify for reliable file watching, then communicates changes to the Windows UI.

## Directory Structure (Planned)

```
intermediary/
├── app/                    # Frontend (React/TS)
│   └── src/
├── src-tauri/              # Tauri backend (Rust)
│   └── src/
├── agent/                  # WSL agent (Rust or Node)
│   └── src/
├── crates/                 # Rust Crates
├── docs/                   # Documentation
├── scripts/                # Build and utility scripts
└── logs/                   # Runtime logs
```

## Key Workflows

1. **File Change → UI Update:** Repo file changes → inotify → WSL agent → WebSocket → UI updates recent list
2. **Drag-out:** User drags row → UI requests staging → Agent copies to staging → UI initiates OS drag with Windows path
3. **Bundle Build:** User clicks Build → UI requests bundle → Agent zips + writes manifest → Agent stages to Windows → UI shows in bundle list

## Related docs

- [docs/prd.md](prd.md) — Full product requirements
