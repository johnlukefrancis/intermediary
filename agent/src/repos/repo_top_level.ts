// Path: agent/src/repos/repo_top_level.ts
// Description: Scan top-level directories and files in a repo

import * as fs from "node:fs/promises";
import { logger } from "../util/logger.js";

/** Directories to exclude from top-level listing */
const EXCLUDED_DIRS = new Set([
  "node_modules",
  ".git",
  "dist",
  "build",
  "target",
  ".vscode",
  ".idea",
]);

export interface TopLevelResult {
  dirs: string[];
  files: string[];
}

/**
 * Scan top-level entries in a repository root.
 * Returns sorted arrays of directory and file names.
 */
export async function getRepoTopLevel(rootPath: string): Promise<TopLevelResult> {
  const dirs: string[] = [];
  const files: string[] = [];

  try {
    const entries = await fs.readdir(rootPath, { withFileTypes: true });

    for (const entry of entries) {
      // Skip hidden files/dirs
      if (entry.name.startsWith(".")) {
        continue;
      }

      if (entry.isDirectory()) {
        if (!EXCLUDED_DIRS.has(entry.name)) {
          dirs.push(entry.name);
        }
      } else if (entry.isFile()) {
        files.push(entry.name);
      }
    }

    dirs.sort();
    files.sort();
  } catch (err) {
    logger.error("Failed to scan repo top level", {
      rootPath,
      error: err instanceof Error ? err.message : String(err),
    });
    throw err;
  }

  return { dirs, files };
}

/**
 * Check if a path exists and is a directory.
 */
export async function isValidRepoRoot(rootPath: string): Promise<boolean> {
  try {
    const stat = await fs.stat(rootPath);
    return stat.isDirectory();
  } catch {
    return false;
  }
}
