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
│  │  File Watcher (inotify)                               │  │
│  │  - Watches configured repo paths                      │  │
│  │  - Emits fileChanged events                           │  │
│  │  - Debounces rapid writes                             │  │
│  └───────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  Staging Service                                      │  │
│  │  - Copies files to Windows staging dir (/mnt/c/...)   │  │
│  │  - Builds zip bundles with manifests                  │  │
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
| WSL Agent | Rust (or Node.js for fast iteration) |
| File Watching | inotify (Linux) via `notify` crate |
| IPC | WebSocket (JSON messages) |
| Drag-out | `tauri-plugin-drag` |

## Project Structure

```
intermediary/
├── app/                    # React/TypeScript frontend
│   ├── src/
│   │   ├── components/     # UI components
│   │   ├── hooks/          # React hooks
│   │   ├── stores/         # State management
│   │   └── types/          # TypeScript types
│   ├── package.json
│   └── tsconfig.json
│
├── src-tauri/              # Tauri Rust backend
│   ├── src/
│   │   ├── commands/       # Tauri command handlers
│   │   ├── config/         # Configuration management
│   │   ├── ipc/            # WebSocket client to agent
│   │   └── staging/        # File staging operations
│   ├── Cargo.toml
│   └── tauri.conf.json
│
├── crates/                 # Shared Rust crates
│   └── intermediary-protocol/  # Shared IPC message types
│
├── agent/                  # WSL agent daemon
│   ├── src/
│   │   ├── watcher/        # inotify file watcher
│   │   ├── bundler/        # Zip bundle builder
│   │   └── server/         # WebSocket server
│   └── Cargo.toml
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
%LOCALAPPDATA%\Intermediary\staging\<repoId>\...
```

The WSL agent writes to this path via `/mnt/c/...`, and the Tauri app reads the same files via their Windows paths.

### Bundle Manifests

Every generated zip includes `INTERMEDIARY_MANIFEST.json`:
```json
{
  "repoId": "my-project",
  "timestamp": "2025-01-15T10:30:00Z",
  "gitShort": "abc1234",
  "dirty": true,
  "changedFiles": ["src/main.ts", "docs/readme.md"],
  "patterns": {
    "include": ["src/**", "docs/**"],
    "exclude": ["**/node_modules/**"]
  },
  "appVersion": "0.1.0"
}
```

### IPC Protocol

Agent → UI:
- `hello` - Agent startup with version info
- `fileChanged` - Single file change event
- `snapshot` - Batch of recent changes
- `bundleBuilt` - Bundle ready for drag-out

UI → Agent:
- `watchRepo` - Start watching a repo
- `stageFile` - Stage a single file for drag-out
- `buildBundle` - Build a zip bundle

## Development

### Prerequisites

- Windows 10/11 with WSL2
- Rust (stable)
- Node.js 20+ with pnpm
- VS Code (recommended)

### Setup

```bash
# Clone the repo (in WSL)
git clone <repo-url> ~/code/intermediary
cd ~/code/intermediary

# Install frontend dependencies
cd app && pnpm install

# Build the agent
cd ../agent && cargo build

# Build the Tauri app
cd ../src-tauri && cargo build
```

### Running

```bash
# Start the WSL agent
cd agent && cargo run

# In another terminal, start the Tauri dev server
cd src-tauri && cargo tauri dev
```

## Status

**Current phase:** Foundation setup (no code yet)

See [docs/roadmap.md](docs/roadmap.md) for development phases.

## License

TBD
