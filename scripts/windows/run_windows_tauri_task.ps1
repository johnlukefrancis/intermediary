# Path: scripts/windows/run_windows_tauri_task.ps1
# Description: PowerShell entrypoint for Windows Tauri VS Code tasks with WSL sync/watch handoff

[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [string]$WindowsMirrorPath,

  [Parameter(Mandatory = $true)]
  [string]$WslRepoPath,

  [Parameter(Mandatory = $true)]
  [string]$WslDistro,

  [Parameter(Mandatory = $true)]
  [ValidateSet("dev", "dev-watch-sync", "build-installer")]
  [string]$Mode,

  [Parameter()]
  [string]$WindowsLocalAppDataWslPath = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$env:INTERMEDIARY_WIN_PATH = $WindowsMirrorPath
$env:INTERMEDIARY_WSL_PATH = $WslRepoPath
$env:INTERMEDIARY_WSL_DISTRO = $WslDistro

if ($Mode -ne "build-installer") {
  $env:INTERMEDIARY_WSL_BACKEND_MODE = "external"
}

$wslPathForUnc = $env:INTERMEDIARY_WSL_PATH.TrimStart("/").Replace("/", "\")
$env:INTERMEDIARY_LOG_DIR = "\\wsl$\$($env:INTERMEDIARY_WSL_DISTRO)\$wslPathForUnc\logs"

function Exit-OnFailure {
  param(
    [Parameter(Mandatory = $true)]
    [int]$ExitCode
  )

  if ($ExitCode -ne 0) {
    exit $ExitCode
  }
}

function Invoke-WslRepoCommand {
  param(
    [Parameter(Mandatory = $true)]
    [string]$CommandText
  )

  & wsl.exe -d $env:INTERMEDIARY_WSL_DISTRO -- bash -lc "cd '$($env:INTERMEDIARY_WSL_PATH)' && $CommandText"
  Exit-OnFailure -ExitCode $LASTEXITCODE
}

function Start-WslRepoProcess {
  param(
    [Parameter(Mandatory = $true)]
    [string]$CommandText
  )

  Start-Process -FilePath "wsl.exe" -ArgumentList @(
    "-d",
    $env:INTERMEDIARY_WSL_DISTRO,
    "--",
    "bash",
    "-lc",
    "cd '$($env:INTERMEDIARY_WSL_PATH)' && $CommandText"
  ) | Out-Null
}

function Wait-ForTcpPort {
  param(
    [Parameter(Mandatory = $true)]
    [int]$Port,

    [Parameter()]
    [int]$TimeoutMs = 15000
  )

  $deadline = (Get-Date).AddMilliseconds($TimeoutMs)
  while ((Get-Date) -lt $deadline) {
    try {
      $client = [System.Net.Sockets.TcpClient]::new()
      $iar = $client.BeginConnect("127.0.0.1", $Port, $null, $null)
      if ($iar.AsyncWaitHandle.WaitOne(250)) {
        $client.EndConnect($iar)
        $client.Dispose()
        return
      }
      $client.Dispose()
    } catch {
    }

    Start-Sleep -Milliseconds 250
  }

  throw "Timed out waiting for 127.0.0.1:$Port"
}

function Invoke-NativeCommand {
  param(
    [Parameter(Mandatory = $true)]
    [string]$FilePath,

    [Parameter()]
    [string[]]$ArgumentList = @()
  )

  & $FilePath @ArgumentList
  Exit-OnFailure -ExitCode $LASTEXITCODE
}

function Start-WslAgentIfNeeded {
  if ($Mode -eq "build-installer") {
    return
  }

  $agentBootstrap = @(
    'export PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"',
    'export INTERMEDIARY_AGENT_PORT=3142',
    "export INTERMEDIARY_WINDOWS_LOCALAPPDATA='$WindowsLocalAppDataWslPath'",
    'set -e',
    'if [ -s "$HOME/.cargo/env" ]; then . "$HOME/.cargo/env"; else echo "cargo env not found at $HOME/.cargo/env" >&2; exit 1; fi',
    'command -v cargo >/dev/null 2>&1 || { echo "cargo not available in PATH" >&2; exit 1; }',
    'bash ./scripts/dev/run_wsl_agent_dev.sh'
  ) -join '; '

  Start-WslRepoProcess -CommandText $agentBootstrap
  Wait-ForTcpPort -Port 3142
}

Start-WslAgentIfNeeded

switch ($Mode) {
  "dev" {
    Invoke-WslRepoCommand -CommandText "./scripts/windows/sync_to_windows.sh"
  }
  "dev-watch-sync" {
    Invoke-WslRepoCommand -CommandText "./scripts/windows/sync_to_windows.sh"
    Start-WslRepoProcess -CommandText "./scripts/windows/watch_sync_to_windows.sh"
  }
  "build-installer" {
    Invoke-WslRepoCommand -CommandText "bash ./scripts/build/build_agent_bundle.sh && ./scripts/windows/sync_to_windows.sh"
  }
}

if (-not (Test-Path $env:INTERMEDIARY_WIN_PATH)) {
  Write-Error "Sync did not create $($env:INTERMEDIARY_WIN_PATH)"
  exit 1
}

Set-Location $env:INTERMEDIARY_WIN_PATH

if (-not (Test-Path "node_modules")) {
  Invoke-NativeCommand -FilePath "pnpm" -ArgumentList @("install")
}

if ($Mode -ne "build-installer") {
  New-Item -ItemType Directory -Force $env:INTERMEDIARY_LOG_DIR | Out-Null
}

Invoke-NativeCommand -FilePath "node" -ArgumentList @("scripts/build/ensure_agent_bundle.mjs")

if (-not (Test-Path "src-tauri/resources/agent_bundle/im_host_agent.exe")) {
  Write-Error "Missing src-tauri/resources/agent_bundle/im_host_agent.exe after ensure_agent_bundle"
  exit 1
}

switch ($Mode) {
  "build-installer" {
    Invoke-NativeCommand -FilePath "pnpm" -ArgumentList @("tauri", "build")
  }
  default {
    Invoke-NativeCommand -FilePath "pnpm" -ArgumentList @("tauri", "dev")
  }
}
