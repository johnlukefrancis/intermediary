# Intermediary

A workflow handoff console for agentic coding workflows. Surfaces recently changed files and generates standardized zip bundles that can be dragged directly into ChatGPT (or anywhere).

## Problem

High-friction file/context handoff between local repos (often in WSL) and ChatGPT/browser-based workflows.

## Solution

A single-window "handoff console" that:
- Watches repos for file changes (works reliably with WSL Linux filesystem)
- Shows recently changed docs and code in separate columns
- Auto-switches between standard and handset layouts by window size (hysteresis-based)
- Generates zip bundles with provenance manifests
- Enables drag-and-drop of files/bundles directly into browser upload zones

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                Host OS (Windows/macOS)                      │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  Tauri App (React/TS + Rust commands)                │  │
│  │  - Docs | Code | Bundles columns                     │  │
│  │  - Native drag-out and host file-manager actions     │  │
│  └───────────────────────────────────────────────────────┘  │
│                            │                                │
│                   WebSocket (localhost)                     │
│                            │                                │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  Host Agent (im_host_agent)                          │  │
│  │  - UI-facing endpoint                                │  │
│  │  - Routes host-native repos locally                  │  │
│  │  - Forwards WSL repos to WSL backend (Windows only)  │  │
│  └───────────────────────────────────────────────────────┘  │
└──────────────────────────────┬──────────────────────────────┘
                               │
                     WSL boundary (Windows only)
                               │
┌─────────────────────────────────────────────────────────────┐
│                    WSL Backend (im_agent)                   │
│  - inotify watching, staging, bundle build, events          │
└─────────────────────────────────────────────────────────────┘
```

## Tech Stack

| Component | Technology |
|-----------|------------|
| Host UI | Tauri v2 |
| Frontend | React + TypeScript |
| Host Agent | Rust (`crates/im_host_agent`) |
| WSL Backend Agent | Rust (`crates/im_agent`, Windows-only backend) |
| File Watching | notify (inotify on Linux / native host FS handling) |
| Bundling | Rust `im_bundle` library (crates/im_bundle) |
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
│   ├── im_host_agent/      # Host routing agent daemon (Rust)
│   ├── im_agent/           # WSL backend agent daemon (Rust)
│   └── im_bundle/          # Bundle library + CLI (scan + zip + manifest)
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

All draggable files originate from a host staging directory:
```
<app_local_data>/staging/
  files/<repoId>/...
  bundles/<repoId>/<presetId>/...
```

On Windows, the WSL backend writes to the host staging directory through its `/mnt/<drive>/...` mirror path.

### Bundle Manifests

Every generated zip includes `BUNDLE_MANIFEST.json`:
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

- Windows 10/11 (WSL2 required for WSL repos) or macOS
- Rust (stable)
- Node.js 20+ with pnpm
- VS Code (recommended)

### Setup & Running

Use the command docs for current, copy-safe steps:
- `docs/commands/dev_windows.md` — Windows development workflow
- `docs/commands/dev_wsl_agent.md` — WSL backend development (Windows+WSL only)
- `docs/commands/zip_bundles.md` — build repo context bundles

## Status

**Current phase:** Host-routed architecture implemented (host agent + optional Windows WSL backend); ongoing parity hardening and documentation alignment.

See [docs/roadmap.md](docs/roadmap.md) for development phases.

## License

TBD
