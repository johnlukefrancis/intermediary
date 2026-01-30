// Path: agent/src/bundles/bundle_builder.ts
// Description: Orchestrates bundle building process

import * as path from "node:path";
import * as fs from "node:fs/promises";
import * as crypto from "node:crypto";
import { type PathBridgeConfig, wslToWindows } from "../staging/path_bridge.js";
import { getGitInfo } from "./git_info.js";
import { createManifest, serializeManifest } from "./manifest.js";
import { writeZip } from "./zip_writer.js";
import { cleanupOldBundles } from "./retention.js";
import type { BuildBundleOptions, BuildBundleResult } from "./bundle_types.js";
import { logger } from "../util/logger.js";

export interface BundleBuilder {
  buildBundle(options: BuildBundleOptions): Promise<BuildBundleResult>;
}

/**
 * Format timestamp for bundle filename
 */
function formatTimestamp(date: Date): string {
  const y = date.getFullYear();
  const mo = String(date.getMonth() + 1).padStart(2, "0");
  const d = String(date.getDate()).padStart(2, "0");
  const h = String(date.getHours()).padStart(2, "0");
  const mi = String(date.getMinutes()).padStart(2, "0");
  const s = String(date.getSeconds()).padStart(2, "0");
  return `${y}${mo}${d}_${h}${mi}${s}`;
}

export function createBundleBuilder(_pathConfig: PathBridgeConfig): BundleBuilder {
  async function buildBundle(options: BuildBundleOptions): Promise<BuildBundleResult> {
    const { repoId, repoRoot, presetId, presetName, selection, outputDir } = options;
    const builtAt = new Date();
    const builtAtIso = builtAt.toISOString();

    // 1. Ensure output directory exists
    const bundleDir = path.posix.join(outputDir, "bundles", repoId, presetId);
    await fs.mkdir(bundleDir, { recursive: true });

    // 2. Get git info (best-effort)
    const gitInfo = await getGitInfo(repoRoot);

    // 3. Generate filename
    const timestamp = formatTimestamp(builtAt);
    const shortSha = gitInfo.shortSha ?? "nosha";
    const fileName = `${repoId}_${presetId}_${timestamp}_${shortSha}.zip`;
    const wslPath = path.posix.join(bundleDir, fileName);

    // 4. Create temp file for atomic write
    const tempPath = wslPath + `.${crypto.randomUUID()}.tmp`;

    // 5. Determine which dirs to include
    const topLevelDirsIncluded =
      selection.topLevelDirs.length > 0
        ? selection.topLevelDirs
        : await getTopLevelDirs(repoRoot);

    // 6. Create manifest
    // We'll update fileCount after zip is written
    const manifest = createManifest(
      repoId,
      repoRoot,
      presetId,
      presetName,
      { includeRoot: selection.includeRoot, topLevelDirsIncluded },
      gitInfo,
      0, // Will be updated
      0  // Will be updated
    );

    try {
      // 7. Write zip with manifest
      const manifestJson = serializeManifest(manifest);
      const zipResult = await writeZip({
        outputPath: tempPath,
        repoRoot,
        includeRoot: selection.includeRoot,
        topLevelDirs: selection.topLevelDirs,
        manifestJson,
      });

      // 8. Atomic rename
      await fs.rename(tempPath, wslPath);

      // 9. Update LATEST alias (copy, not rename)
      const aliasFileName = `${repoId}_${presetId}_LATEST.zip`;
      const aliasWslPath = path.posix.join(bundleDir, aliasFileName);
      await fs.copyFile(wslPath, aliasWslPath);

      // 10. Cleanup old bundles
      await cleanupOldBundles({ bundleDir, repoId, presetId, keepCount: 10 });

      // 11. Convert paths to Windows
      const windowsPath = wslToWindows(wslPath);
      const aliasWindowsPath = wslToWindows(aliasWslPath);

      logger.info("Bundle built", {
        repoId,
        presetId,
        fileCount: zipResult.fileCount,
        bytes: zipResult.bytes,
      });

      return {
        wslPath,
        windowsPath,
        aliasWslPath,
        aliasWindowsPath,
        bytes: zipResult.bytes,
        fileCount: zipResult.fileCount,
        builtAtIso,
      };
    } catch (err) {
      // Cleanup temp file on failure
      try {
        await fs.unlink(tempPath);
      } catch {
        // Ignore cleanup errors
      }
      throw err;
    }
  }

  return { buildBundle };
}

/**
 * Get all non-ignored top-level directories
 */
async function getTopLevelDirs(repoRoot: string): Promise<string[]> {
  const { shouldIgnoreEntry } = await import("./ignore_rules.js");
  const entries = await fs.readdir(repoRoot, { withFileTypes: true });
  return entries
    .filter((e) => e.isDirectory() && !shouldIgnoreEntry(e.name, true))
    .map((e) => e.name)
    .sort();
}
