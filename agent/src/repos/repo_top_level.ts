// Path: agent/src/repos/repo_top_level.ts
// Description: Scan top-level directories and files in a repo

import * as fs from "node:fs/promises";
import { logger } from "../util/logger.js";
import { shouldIgnoreEntry } from "../bundles/ignore_rules.js";

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
      if (entry.isDirectory()) {
        if (!shouldIgnoreEntry(entry.name, true)) {
          dirs.push(entry.name);
        }
      } else if (entry.isFile()) {
        if (!shouldIgnoreEntry(entry.name, false)) {
          files.push(entry.name);
        }
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
