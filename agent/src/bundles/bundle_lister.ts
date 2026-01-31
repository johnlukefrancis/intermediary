// Path: agent/src/bundles/bundle_lister.ts
// Description: Find the single bundle file for a preset

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
 * Find the single bundle for a given repo+preset
 * Returns single-item array if exists, empty array otherwise
 */
export async function listBundles(options: ListBundlesOptions): Promise<BundleListEntry[]> {
  const { bundleDir, repoId, presetId } = options;
  const targetDir = path.posix.join(bundleDir, "bundles", repoId, presetId);
  const prefix = `${repoId}_${presetId}_`;

  try {
    await fs.access(targetDir);
  } catch {
    return [];
  }

  try {
    const entries = await fs.readdir(targetDir);

    // Find the single matching bundle (should only be one)
    for (const fileName of entries) {
      if (!fileName.startsWith(prefix) || !fileName.endsWith(".zip")) continue;

      const wslPath = path.posix.join(targetDir, fileName);
      try {
        const stat = await fs.stat(wslPath);
        return [
          {
            wslPath,
            windowsPath: wslToWindows(wslPath),
            fileName,
            bytes: stat.size,
            mtimeMs: stat.mtimeMs,
            isLatestAlias: true, // Always true since there's only one
          },
        ];
      } catch {
        logger.debug("Could not stat bundle", { fileName });
      }
    }

    return [];
  } catch (err) {
    logger.debug("Could not list bundle directory", {
      targetDir,
      error: err instanceof Error ? err.message : String(err),
    });
    return [];
  }
}
