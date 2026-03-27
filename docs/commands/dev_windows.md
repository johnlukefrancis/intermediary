# Windows Development Commands
Updated on: 2026-02-12
Owners: JL · Agents
Depends on: ADR-000, ADR-012

Commands for running Intermediary in Windows with WSL source files.

This is the maintainer-validated development path for the repo. macOS and Linux are not yet validated to the same standard.

## VS Code Tasks (Recommended)

The project includes VS Code tasks that handle sync and build automatically.
The tracked task file is publication-safe: it prompts for your Windows mirror path, WSL distro, and optional `/mnt/...` LocalAppData path instead of shipping machine-specific values.

Note: Installed builds auto-start the host agent (`3141`) and conditionally start the WSL backend (`3142`) when WSL repos are configured. During development these tasks launch the WSL backend from source on `3142`; the host remains app-managed on `3141`.
When `INTERMEDIARY_WSL_WS_TOKEN` is not explicitly set, the WSL dev launcher resolves `wslWsToken` from the app auth file (`ws_auth.json`) under app local data so backend token auth matches the running app by default.
Windows dev tasks that launch `WSL Agent: dev` now set `INTERMEDIARY_WSL_BACKEND_MODE=external` so the app treats port `3142` as externally managed and avoids in-distro terminate loops.
`WSL Agent: dev` also accepts an optional `INTERMEDIARY_WINDOWS_LOCALAPPDATA` input so token discovery can stay deterministic even when WSL PATH does not expose Windows interop binaries.

Important:
- For repos rooted on mounted Windows paths (`/mnt/<drive>/...`), prefer the Windows tasks in this document.
- Running Linux-target Tauri directly from WSL for those roots can produce degraded watch reliability; Intermediary now warns when this mode is detected.

### Tauri: dev (Windows)

Syncs from WSL, installs dependencies if needed, then starts Tauri dev server.
Also launches the WSL backend agent in a separate WSL process on `3142`.
Before launch, it runs `scripts/build/ensure_agent_bundle.mjs` on Windows so `im_host_agent.exe` is present/refreshed in `src-tauri/resources/agent_bundle`.

Task name: `Tauri: dev (Windows)`

This is the **default build task** (Ctrl+Shift+B).

### Tauri: dev (Windows, watch + sync)

Same as above, plus starts a background watcher that continuously syncs changes from WSL.
Also launches the WSL backend agent in a separate WSL process on `3142`.
This task also runs host-bundle prep (`scripts/build/ensure_agent_bundle.mjs`) before launching Tauri.

Task name: `Tauri: dev (Windows, watch + sync)`

### Tauri: build installer (Windows)

Syncs from WSL and builds the production installer.
Runs the WSL agent bundle script (`scripts/build/build_agent_bundle.sh`) before syncing so
the packaged resources include the latest Rust agent binary without requiring Node in WSL.
Then runs `scripts/build/ensure_agent_bundle.mjs` on Windows to refresh/verify `im_host_agent.exe` before `pnpm tauri build`.

Task name: `Tauri: build installer (Windows)`

## Manual Workflow

If you need to run commands manually:

### 1. Sync WSL to Windows

From WSL, sync source files to the Windows mirror directory:

```bash
./scripts/windows/sync_to_windows.sh
```

### 2. Install dependencies (Windows)

From the Windows mirror directory (for example `D:\code\intermediary`):

```powershell
pnpm install
```

### 3. Start dev server (Windows)

From the Windows directory:

```powershell
$env:INTERMEDIARY_LOG_DIR='\\wsl$\<your-distro>\<your-wsl-path>\logs'
pnpm tauri dev
```

### 4. Start WSL backend agent (WSL)

In a separate WSL terminal:

```bash
pnpm run agent:dev
```

The launcher resolves `INTERMEDIARY_WSL_WS_TOKEN` in this order:
1. Explicit `INTERMEDIARY_WSL_WS_TOKEN` environment variable
2. `wslWsToken` from `ws_auth.json` under the active Windows `%LOCALAPPDATA%` profile (`com.johnf.intermediary/agent/` then legacy `Intermediary/agent/`)
3. Fallback dev token (`im_dev_wsl_token`) with a warning (this usually indicates auth drift and will cause websocket `invalid_token` failures until corrected)

## Environment Variables

The VS Code tasks set these automatically:

| Variable | Description | Example |
|----------|-------------|---------|
| `INTERMEDIARY_WIN_PATH` | Windows mirror directory | `D:\code\intermediary` |
| `INTERMEDIARY_WSL_PATH` | WSL source directory | `/home/<you>/code/intermediary` |
| `INTERMEDIARY_WSL_DISTRO` | WSL distro for VS Code tasks (sync scripts) | `Ubuntu` |
| `INTERMEDIARY_WSL_BACKEND_MODE` | WSL backend ownership mode in app runtime (`external` in Windows dev tasks) | `external` |
| `INTERMEDIARY_WINDOWS_LOCALAPPDATA` | Deterministic Windows app-local path used by WSL agent launcher to resolve `ws_auth.json` | `/mnt/c/Users/<you>/AppData/Local` |
| `INTERMEDIARY_LOG_DIR` | Log output directory (WSL UNC path) | `\\wsl$\<your-distro>\<your-wsl-path>\logs` |

**Note:** Native WSL paths (e.g., `/home/...`) are converted to Windows paths automatically via `wslpath` at runtime. Intermediary now uses persisted app config (`agentDistro`) as the primary distro authority for runtime conversion; `INTERMEDIARY_WSL_DISTRO` is mainly for dev scripts/tools and as a fallback when no app override is configured.

## Watch Sync

The watch sync script monitors WSL for changes and copies them to Windows:

```bash
./scripts/windows/watch_sync_to_windows.sh
```

This is started automatically by the "Tauri: dev (Windows, watch + sync)" task.
The sync scripts intentionally exclude `src-tauri/resources/agent_bundle/im_host_agent.exe` so WSL mirror updates do not delete the Windows-built host agent binary.

## Typical Workflow

1. Open VS Code in WSL (`code .` from your WSL repo root)
2. Press Ctrl+Shift+B to run "Tauri: dev (Windows)"
3. In a separate terminal, run `pnpm run agent:dev` to start the WSL backend on `3142`
4. Edit code in VS Code; Tauri hot-reloads the frontend
5. For Rust changes, save and let Tauri rebuild

## Notes

- **Always edit in WSL**, not in the Windows mirror directory
- The Windows mirror is read-only from your perspective; sync scripts overwrite it
- Logs appear in `logs/run_latest.txt` (backend) and the terminal (agent tasks)
