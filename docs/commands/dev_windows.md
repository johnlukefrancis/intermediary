# Windows Development Commands
Updated on: 2026-02-03
Owners: JL · Agents
Depends on: ADR-000, ADR-012

Commands for running Intermediary in Windows with WSL source files.

## VS Code Tasks (Recommended)

The project includes VS Code tasks that handle sync and build automatically.

### Tauri: dev (Windows)

Syncs from WSL, installs dependencies if needed, then starts Tauri dev server.
Also launches the WSL agent in a separate WSL process.

Task name: `Tauri: dev (Windows)`

This is the **default build task** (Ctrl+Shift+B).

### Tauri: dev (Windows, watch + sync)

Same as above, plus starts a background watcher that continuously syncs changes from WSL.
Also launches the WSL agent in a separate WSL process.

Task name: `Tauri: dev (Windows, watch + sync)`

### Tauri: build installer (Windows)

Syncs from WSL and builds the production installer.

Task name: `Tauri: build installer (Windows)`

## Manual Workflow

If you need to run commands manually:

### 1. Sync WSL to Windows

From WSL, sync source files to the Windows mirror directory:

```bash
./scripts/windows/sync_to_windows.sh
```

### 2. Install dependencies (Windows)

From the Windows directory (D:\code\intermediary by default):

```powershell
pnpm install
```

### 3. Start dev server (Windows)

From the Windows directory:

```powershell
$env:INTERMEDIARY_LOG_DIR='\\wsl$\Ubuntu\home\johnf\code\intermediary\logs'
pnpm tauri dev
```

### 4. Start WSL agent (WSL)

In a separate WSL terminal:

```bash
pnpm run agent:dev
```

## Environment Variables

The VS Code tasks set these automatically:

| Variable | Description | Example |
|----------|-------------|---------|
| `INTERMEDIARY_WIN_PATH` | Windows mirror directory | `D:\code\intermediary` |
| `INTERMEDIARY_WSL_PATH` | WSL source directory | `/home/johnf/code/intermediary` |
| `INTERMEDIARY_WSL_DISTRO` | WSL distro for VS Code tasks (sync scripts) | `Ubuntu` |
| `INTERMEDIARY_LOG_DIR` | Log output directory (WSL UNC path) | `\\wsl$\Ubuntu\home\johnf\code\intermediary\logs` |

**Note:** Native WSL paths (e.g., `/home/...`) are converted to Windows paths automatically via `wslpath` at runtime. No environment variable configuration is needed for most setups; if your repos live in a non-default distro, set `INTERMEDIARY_WSL_DISTRO` so `wslpath` targets that distro.

## Watch Sync

The watch sync script monitors WSL for changes and copies them to Windows:

```bash
./scripts/windows/watch_sync_to_windows.sh
```

This is started automatically by the "Tauri: dev (Windows, watch + sync)" task.

## Typical Workflow

1. Open VS Code in WSL (`code .` from `/home/johnf/code/intermediary`)
2. Press Ctrl+Shift+B to run "Tauri: dev (Windows)"
3. In a separate terminal, run `pnpm run agent:dev` to start the WSL agent
4. Edit code in VS Code; Tauri hot-reloads the frontend
5. For Rust changes, save and let Tauri rebuild

## Notes

- **Always edit in WSL**, not in the Windows mirror directory
- The Windows mirror is read-only from your perspective; sync scripts overwrite it
- Logs appear in `logs/run_latest.txt` (backend) and the terminal (agent)
