// Path: agent/src/repos/repo_top_level.ts
// Description: Scan top-level directories and files in a repo

import * as fs from "node:fs/promises";
import * as path from "node:path";
import { logger } from "../util/logger.js";
import { shouldIgnoreEntry } from "../bundles/ignore_rules.js";

export interface TopLevelResult {
  dirs: string[];
  files: string[];
  /** Subdirectories within each top-level dir (depth-2) */
  subdirs: Record<string, string[]>;
}

/**
 * Scan top-level entries in a repository root.
 * Returns sorted arrays of directory and file names, plus depth-2 subdirs.
 */
export async function getRepoTopLevel(rootPath: string): Promise<TopLevelResult> {
  const dirs: string[] = [];
  const files: string[] = [];
  const subdirs: Record<string, string[]> = {};

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

    // Scan depth-2 subdirectories for each top-level dir
    for (const dir of dirs) {
      try {
        const dirPath = path.join(rootPath, dir);
        const subEntries = await fs.readdir(dirPath, { withFileTypes: true });
        subdirs[dir] = subEntries
          .filter((e) => e.isDirectory() && !shouldIgnoreEntry(e.name, true))
          .map((e) => e.name)
          .sort();
      } catch {
        // Skip directories we can't read
        subdirs[dir] = [];
      }
    }
  } catch (err) {
    logger.error("Failed to scan repo top level", {
      rootPath,
      error: err instanceof Error ? err.message : String(err),
    });
    throw err;
  }

  return { dirs, files, subdirs };
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
