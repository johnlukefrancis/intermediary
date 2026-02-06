// Path: app/src/types/app_paths.ts
// Description: TypeScript interface matching Rust AppPaths struct

/**
 * Application paths resolved by Tauri backend.
 * Matches the Rust AppPaths struct in src-tauri/src/lib/paths/app_paths.rs
 * Note: Rust uses snake_case but serde renames to camelCase for JSON
 */
export interface AppPaths {
  /** Host app-local-data directory for this app */
  appLocalDataDir: string;
  /** Host-native path to staging root */
  stagingHostRoot: string;
  /** Optional WSL equivalent path to staging root (Windows + WSL only) */
  stagingWslRoot?: string;
  /** Log directory path */
  logDir: string;
  /** Path to drag icon PNG */
  dragIconHostPath: string;
}
