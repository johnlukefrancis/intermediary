// Path: agent/src/bundles/retention.ts
// Description: Bundle cleanup logic (keep last N)

import * as fs from "node:fs/promises";
import * as path from "node:path";
import { logger } from "../util/logger.js";

const DEFAULT_RETENTION_COUNT = 10;

export interface RetentionOptions {
  bundleDir: string;
  repoId: string;
  presetId: string;
  keepCount?: number;
}

interface BundleFile {
  name: string;
  fullPath: string;
  mtimeMs: number;
}

/**
 * Clean up old bundles, keeping only the most recent N
 * Never deletes the LATEST alias
 * Returns number of bundles deleted
 */
export async function cleanupOldBundles(options: RetentionOptions): Promise<number> {
  const { bundleDir, repoId, presetId, keepCount = DEFAULT_RETENTION_COUNT } = options;
  const prefix = `${repoId}_${presetId}_`;
  const latestAlias = `${repoId}_${presetId}_LATEST.zip`;

  try {
    const entries = await fs.readdir(bundleDir);
    const bundles: BundleFile[] = [];

    for (const name of entries) {
      // Skip non-matching files
      if (!name.startsWith(prefix) || !name.endsWith(".zip")) continue;
      // Never delete LATEST alias
      if (name === latestAlias) continue;

      const fullPath = path.join(bundleDir, name);
      try {
        const stat = await fs.stat(fullPath);
        bundles.push({ name, fullPath, mtimeMs: stat.mtimeMs });
      } catch {
        // Skip files that can't be stat'd
      }
    }

    // Sort by mtime descending (newest first)
    bundles.sort((a, b) => b.mtimeMs - a.mtimeMs);

    // Delete bundles beyond keepCount
    let deleted = 0;
    const toDelete = bundles.slice(keepCount);
    for (const bundle of toDelete) {
      try {
        await fs.unlink(bundle.fullPath);
        deleted++;
        logger.debug("Deleted old bundle", { name: bundle.name });
      } catch (err) {
        logger.warn("Failed to delete old bundle", {
          name: bundle.name,
          error: err instanceof Error ? err.message : String(err),
        });
      }
    }

    return deleted;
  } catch (err) {
    logger.warn("Bundle cleanup failed", {
      bundleDir,
      error: err instanceof Error ? err.message : String(err),
    });
    return 0;
  }
}
