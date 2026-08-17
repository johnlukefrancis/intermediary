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
  [ValidateSet("dev", "dev-watch-sync", "build-installer", "build-installer-launch")]
  [string]$Mode
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$env:INTERMEDIARY_WIN_PATH = $WindowsMirrorPath
$env:INTERMEDIARY_WSL_PATH = $WslRepoPath
$env:INTERMEDIARY_WSL_DISTRO = $WslDistro

$wslEnvEntries = @(
  $env:WSLENV -split ":" |
    Where-Object {
      -not [string]::IsNullOrWhiteSpace($_) -and
      $_ -notmatch "^INTERMEDIARY_WIN_PATH(?:/.*)?$"
    }
)
$env:WSLENV = ($wslEnvEntries + "INTERMEDIARY_WIN_PATH/up") -join ":"

$isInstallerMode = $Mode -in @("build-installer", "build-installer-launch")

if (-not $isInstallerMode) {
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

function Resolve-FreshNsisInstaller {
  param(
    [Parameter(Mandatory = $true)]
    [datetime]$BuildStartedUtc
  )

  $configPath = Join-Path $env:INTERMEDIARY_WIN_PATH "src-tauri/tauri.conf.json"
  if (-not (Test-Path -LiteralPath $configPath -PathType Leaf)) {
    throw "Tauri config not found after sync: $configPath"
  }
  $config = Get-Content -LiteralPath $configPath -Raw | ConvertFrom-Json
  $productName = [string]$config.productName
  $version = [string]$config.version
  if ([string]::IsNullOrWhiteSpace($productName) -or [string]::IsNullOrWhiteSpace($version)) {
    throw "Tauri config must define productName and version before installer resolution"
  }

  $nsisDir = Join-Path $env:INTERMEDIARY_WIN_PATH "target/release/bundle/nsis"
  if (-not (Test-Path -LiteralPath $nsisDir -PathType Container)) {
    throw "NSIS output directory was not created: $nsisDir"
  }
  $pattern = "$($productName)_$($version)_*-setup.exe"
  $minimumWriteTime = $BuildStartedUtc.AddSeconds(-2)
  $installers = @(
    Get-ChildItem -LiteralPath $nsisDir -Filter $pattern -File |
      Where-Object { $_.LastWriteTimeUtc -ge $minimumWriteTime }
  )
  if ($installers.Count -ne 1) {
    throw "Expected exactly one freshly built NSIS installer matching '$pattern'; found $($installers.Count)"
  }

  return $installers[0]
}

if ($isInstallerMode) {
  Invoke-WslRepoCommand -CommandText "bash ./scripts/build/build_agent_bundle.sh && ./scripts/windows/sync_to_windows.sh"
} else {
  switch ($Mode) {
    "dev" {
      Invoke-WslRepoCommand -CommandText "./scripts/windows/sync_to_windows.sh"
    }
    "dev-watch-sync" {
      Invoke-WslRepoCommand -CommandText "./scripts/windows/sync_to_windows.sh"
      Start-WslRepoProcess -CommandText "./scripts/windows/watch_sync_to_windows.sh"
    }
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

if (-not $isInstallerMode) {
  New-Item -ItemType Directory -Force $env:INTERMEDIARY_LOG_DIR | Out-Null
}

Invoke-NativeCommand -FilePath "node" -ArgumentList @("scripts/build/ensure_agent_bundle.mjs")

if (-not (Test-Path "src-tauri/resources/agent_bundle/im_host_agent.exe")) {
  Write-Error "Missing src-tauri/resources/agent_bundle/im_host_agent.exe after ensure_agent_bundle"
  exit 1
}

if ($isInstallerMode) {
  $buildStartedUtc = [DateTime]::UtcNow
  Invoke-NativeCommand -FilePath "pnpm" -ArgumentList @("tauri", "build")
  if ($Mode -eq "build-installer-launch") {
    $installer = Resolve-FreshNsisInstaller -BuildStartedUtc $buildStartedUtc
    Write-Host "Launching freshly built installer: $($installer.FullName)"
    Start-Process -FilePath $installer.FullName | Out-Null
  }
} else {
  Invoke-NativeCommand -FilePath "pnpm" -ArgumentList @("tauri", "dev")
}
