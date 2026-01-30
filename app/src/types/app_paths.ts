// Path: app/src/types/app_paths.ts
// Description: TypeScript interface matching Rust AppPaths struct

/**
 * Application paths resolved by Tauri backend.
 * Matches the Rust AppPaths struct in src-tauri/src/lib/paths/app_paths.rs
 * Note: Rust uses snake_case but serde renames to camelCase for JSON
 */
export interface AppPaths {
  /** Windows AppData\Local directory for this app */
  appLocalDataDir: string;
  /** Windows path to staging root */
  stagingWindowsRoot: string;
  /** WSL equivalent path to staging root */
  stagingWslRoot: string;
  /** Log directory path */
  logDir: string;
  /** Path to drag icon PNG */
  dragIconWindowsPath: string;
}
