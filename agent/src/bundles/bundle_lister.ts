// Path: agent/src/bundles/bundle_lister.ts
// Description: List existing bundles for a preset

import * as fs from "node:fs/promises";
import * as path from "node:path";
import { wslToWindows } from "../staging/path_bridge.js";
import { logger } from "../util/logger.js";

export interface BundleListEntry {
  wslPath: string;
  windowsPath: string;
  fileName: string;
  bytes: number;
  mtimeMs: number;
  isLatestAlias: boolean;
}

export interface ListBundlesOptions {
  bundleDir: string;
  repoId: string;
  presetId: string;
}

/**
 * List all bundles for a given repo+preset
 * Returns LATEST alias first, then others sorted by mtime descending
 */
export async function listBundles(options: ListBundlesOptions): Promise<BundleListEntry[]> {
  const { bundleDir, repoId, presetId } = options;
  const targetDir = path.posix.join(bundleDir, "bundles", repoId, presetId);
  const prefix = `${repoId}_${presetId}_`;
  const latestAlias = `${repoId}_${presetId}_LATEST.zip`;

  const results: BundleListEntry[] = [];
  let latestEntry: BundleListEntry | null = null;

  try {
    await fs.access(targetDir);
  } catch {
    // Directory doesn't exist yet, return empty
    return [];
  }

  try {
    const entries = await fs.readdir(targetDir);

    for (const fileName of entries) {
      if (!fileName.startsWith(prefix) || !fileName.endsWith(".zip")) continue;

      const wslPath = path.posix.join(targetDir, fileName);
      try {
        const stat = await fs.stat(wslPath);
        const entry: BundleListEntry = {
          wslPath,
          windowsPath: wslToWindows(wslPath),
          fileName,
          bytes: stat.size,
          mtimeMs: stat.mtimeMs,
          isLatestAlias: fileName === latestAlias,
        };

        if (fileName === latestAlias) {
          latestEntry = entry;
        } else {
          results.push(entry);
        }
      } catch {
        logger.debug("Could not stat bundle", { fileName });
      }
    }

    // Sort by mtime descending (newest first)
    results.sort((a, b) => b.mtimeMs - a.mtimeMs);

    // Put LATEST alias first if it exists
    if (latestEntry) {
      results.unshift(latestEntry);
    }

    return results;
  } catch (err) {
    logger.warn("Failed to list bundles", {
      targetDir,
      error: err instanceof Error ? err.message : String(err),
    });
    return [];
  }
}
