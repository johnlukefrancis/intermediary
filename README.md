# Intermediary

A workflow handoff console for agentic coding workflows. Surfaces recently changed files and generates standardized zip bundles that can be dragged directly into ChatGPT (or anywhere).

## Problem

High-friction file/context handoff between local repos (often in WSL) and ChatGPT/browser-based workflows.

## Solution

A single-window "handoff console" that:
- Watches repos for file changes (works reliably with WSL Linux filesystem)
- Shows recently changed docs and code in separate columns
- Generates zip bundles with provenance manifests
- Enables drag-and-drop of files/bundles directly into browser upload zones

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Windows (Tauri App)                      │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  React/TS Frontend                                    │  │
│  │  - Tab bar (one per repo)                             │  │
│  │  - Docs column | Code column | Bundles column         │  │
│  │  - Drag handles for each file row                     │  │
│  └───────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  Rust Backend (Tauri commands)                        │  │
│  │  - Config management                                  │  │
│  │  - Staging operations                                 │  │
│  │  - Native drag-out via tauri-plugin-drag              │  │
│  │  - WebSocket client to WSL agent                      │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                            │
                   WebSocket (localhost)
                            │
┌─────────────────────────────────────────────────────────────┐
│                       WSL (Agent)                           │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  Node/TS Agent                                        │  │
│  │  - chokidar file watchers (inotify on Linux)          │  │
│  │  - Emits fileChanged/snapshot events                  │  │
│  │  - Stages files + builds bundles                      │  │
│  │  - Atomic writes (temp + rename)                      │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Tech Stack

| Component | Technology |
|-----------|------------|
| Windows UI | Tauri v2 |
| Frontend | React + TypeScript |
| Backend | Rust |
| WSL Agent | Node.js + TypeScript |
| File Watching | chokidar (inotify on Linux) |
| Bundling | Rust `im_zip_cli` (crates/im_zip) |
| IPC | WebSocket (JSON request/response + events) |
| Drag-out | `tauri-plugin-drag` |

## Project Structure

```
intermediary/
├── app/                    # React/TypeScript frontend
│   ├── src/
│   │   ├── components/     # UI components
│   │   ├── hooks/          # React hooks
│   │   ├── lib/            # Agent client + helpers
│   │   ├── shared/         # Protocol + config types
│   │   ├── styles/         # CSS
│   │   ├── tabs/           # Repo tab UI
│   │   └── types/          # App-facing types
│   ├── package.json
│   └── tsconfig.json
│
├── src-tauri/              # Tauri Rust backend
│   ├── src/
│   │   └── lib/
│   │       ├── commands/   # Tauri command handlers
│   │       ├── config/     # Configuration management
│   │       ├── obs/        # Observability/logging
│   │       └── paths/      # Path resolution + WSL conversion
│   ├── Cargo.toml
│   └── tauri.conf.json
│
├── crates/                 # Rust workspace crates
│   └── im_zip/              # Rust zip CLI + library (bundle writing)
│
├── agent/                  # WSL agent daemon
│   ├── src/
│   │   ├── bundles/        # Bundle building + manifest
│   │   ├── repos/          # File watching
│   │   ├── server/         # WebSocket server + router
│   │   ├── staging/        # Path bridge + file staging
│   │   └── util/           # Logger + helpers
│
├── docs/                   # Documentation
│   ├── compliance/         # ADRs (architectural decisions)
│   ├── environment/        # Dev environment guides
│   ├── inventory/          # File/skill inventories
│   ├── guide.md            # Documentation index
│   ├── prd.md              # Product requirements
│   ├── system_overview.md  # Architecture overview
│   └── roadmap.md          # Development roadmap
│
├── scripts/                # Build and utility scripts
│   ├── zip/                # Bundle generation
│   ├── fileledger/         # File inventory tools
│   └── windows/            # WSL↔Windows sync
│
├── logs/                   # Runtime logs (gitignored)
├── .vscode/                # VS Code tasks and settings
├── CLAUDE.md               # Agent instructions
└── README.md               # This file
```

## Key Concepts

### Staging Directory

All draggable files originate from a staging directory on the Windows filesystem:
```
%LOCALAPPDATA%\Intermediary\staging\
  files\<repoId>\...
  bundles\<repoId>\<presetId>\...
```

The WSL agent writes to this path via `/mnt/c/...`, and the Tauri app reads the same files via their Windows paths.

### Bundle Manifests

Every generated zip includes `INTERMEDIARY_MANIFEST.json`:
```json
{
  "generatedAt": "2026-01-30T10:30:00Z",
  "repoId": "textureportal",
  "repoRoot": "/home/johnf/code/textureportal",
  "presetId": "full",
  "presetName": "Full",
  "selection": {
    "includeRoot": true,
    "topLevelDirsIncluded": ["app", "src-tauri", "docs"],
    "excludedSubdirs": ["TriangleRain/Assets"]
  },
  "git": {
    "headSha": "abc1234",
    "shortSha": "abc1234",
    "branch": "main"
  },
  "fileCount": 312,
  "totalBytesBestEffort": 4820194
}
```

### Bundle Naming + Retention

Bundles are timestamped to make the "latest" obvious, and only the most recent bundle per preset is kept. Older bundles for the same repo + preset are deleted before a new one is written.

### IPC Protocol

Agent → UI:
- `fileChanged` - Single file change event (includes changeType + optional staged info)
- `snapshot` - Batch of recent changes
- `bundleBuilt` - Bundle ready for drag-out (includes alias path + size metadata)
- `error` - Structured error event

UI → Agent:
- `clientHello` - Configure agent with config + staging roots
- `setOptions` - Toggle auto-stage at runtime
- `watchRepo` - Start watching a repo
- `refresh` - Request a fresh snapshot
- `stageFile` - Stage a single file for drag-out
- `buildBundle` - Build a zip bundle (with selection payload)
- `getRepoTopLevel` - Fetch top-level dirs/files
- `listBundles` - List existing bundles for a preset

## Development

### Prerequisites

- Windows 10/11 with WSL2
- Rust (stable)
- Node.js 20+ with pnpm
- VS Code (recommended)

### Setup & Running

Use the command docs for current, copy-safe steps:
- `docs/commands/dev_wsl_agent.md` — start the WSL agent
- `docs/commands/dev_windows.md` — run the Windows UI (Tauri)
- `docs/commands/zip_bundles.md` — build repo context bundles

## Status

**Current phase:** Core app + WSL agent implemented; documentation alignment in progress.

See [docs/roadmap.md](docs/roadmap.md) for development phases.

## License

TBD
