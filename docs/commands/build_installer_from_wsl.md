# Build and Install the Windows Installer From WSL
Updated on: 2026-09-03
Owners: JL · Agents
Depends on: ADR-000, ADR-012

Drive the same PowerShell entrypoint the VS Code installer task uses, but from a WSL shell, so an agent
session can rebuild and reinstall the local Windows app without VS Code. The script builds the Linux
`im_agent` bundle in WSL, syncs the mirror, builds `im_host_agent.exe`, runs `pnpm tauri build`, and
leaves the NSIS installer under the mirror's `target/release/bundle/nsis/`.

## Prerequisites

- `pwsh` is the Windows PowerShell 7 wrapper on PATH in WSL (`~/.local/bin/pwsh`).
- The Windows mirror is `D:\code\intermediary` (the default in `.vscode/tasks.json`); the WSL distro is `Ubuntu`.

## Build the installer

Run from the repo root in WSL (the working directory only matters for the wrapper):

```bash
pwsh -NoProfile -ExecutionPolicy Bypass -Command '& "\\wsl$\Ubuntu\home\johnf\dev\intermediary\scripts\windows\run_windows_tauri_task.ps1" -WindowsMirrorPath "D:\code\intermediary" -WslRepoPath "/home/johnf/dev/intermediary" -WslDistro "Ubuntu" -Mode "build-installer"'
```

The installer lands at `D:\code\intermediary\target\release\bundle\nsis\Intermediary_<version>_x64-setup.exe`.

## Install silently and relaunch

Close the running app first so the installer can replace its files, then install per-user and launch:

```bash
pwsh -NoProfile -Command "Get-Process intermediary -ErrorAction SilentlyContinue | ForEach-Object { \$null = \$_.CloseMainWindow(); \$_.WaitForExit(20000) }; Start-Process -Wait -FilePath (Get-ChildItem 'D:\code\intermediary\target\release\bundle\nsis' -Filter 'Intermediary_*-setup.exe' | Sort-Object LastWriteTimeUtc -Descending | Select-Object -First 1).FullName -ArgumentList '/S'; Start-Process 'C:\Users\Johnf\AppData\Local\Intermediary\intermediary.exe'"
```

## Notes

- `pnpm` 11 refuses to run scripts until build-script approvals are recorded; `pnpm-workspace.yaml`
  carries `allowBuilds` for `esbuild` and `sharp` so `pnpm exec` works in both WSL and the mirror.
- A debug `im_agent` hashes its own (~94 MB) binary at startup and takes ~17 s to start listening; release
  builds take well under a second.
