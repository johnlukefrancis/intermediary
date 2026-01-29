// Path: agent/src/staging/path_bridge.ts
// Description: WSL to Windows path conversion for staging files

import * as path from "node:path";

/**
 * Configuration for path bridge operations.
 */
export interface PathBridgeConfig {
  /** WSL staging root, e.g. /mnt/c/Users/johnf/AppData/Local/Intermediary/staging/files */
  stagingWslRoot: string;
  /** Windows staging root, e.g. C:\Users\johnf\AppData\Local\Intermediary\staging\files */
  stagingWinRoot: string;
}

/**
 * Convert a WSL path under /mnt/c/... to Windows path C:\...
 */
export function wslToWindows(wslPath: string): string {
  // Match /mnt/<drive>/...
  const match = /^\/mnt\/([a-z])\/(.*)$/.exec(wslPath);
  if (!match) {
    throw new Error(`Invalid WSL path for Windows conversion: ${wslPath}`);
  }
  const [, drive, rest] = match;
  // Convert forward slashes to backslashes
  const winPath = rest?.replace(/\//g, "\\") ?? "";
  return `${drive?.toUpperCase()}:\\${winPath}`;
}

/**
 * Convert a Windows path C:\... to WSL path /mnt/c/...
 */
export function windowsToWsl(winPath: string): string {
  // Match C:\... or c:\...
  const match = /^([a-zA-Z]):\\(.*)$/.exec(winPath);
  if (!match) {
    throw new Error(`Invalid Windows path for WSL conversion: ${winPath}`);
  }
  const [, drive, rest] = match;
  // Convert backslashes to forward slashes
  const wslPath = rest?.replace(/\\/g, "/") ?? "";
  return `/mnt/${drive?.toLowerCase()}/${wslPath}`;
}

/**
 * Build the staged file paths given a repo ID and relative path.
 */
export function buildStagedPaths(
  config: PathBridgeConfig,
  repoId: string,
  relativePath: string
): { wslPath: string; windowsPath: string } {
  const normalizedRelative = relativePath.replace(/\\/g, "/");
  const wslPath = path.posix.join(config.stagingWslRoot, repoId, normalizedRelative);
  const windowsPath = `${config.stagingWinRoot}\\${repoId}\\${normalizedRelative.replace(/\//g, "\\")}`;
  return { wslPath, windowsPath };
}
