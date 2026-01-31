// Path: agent/src/bundles/bundle_builder.ts
// Description: Orchestrates bundle building process (single timestamped file, no accumulation)

import * as path from "node:path";
import * as fs from "node:fs/promises";
import * as crypto from "node:crypto";
import { type PathBridgeConfig, wslToWindows } from "../staging/path_bridge.js";
import { getGitInfo } from "./git_info.js";
import type { BuildBundleOptions, BuildBundleResult } from "./bundle_types.js";
import { logger } from "../util/logger.js";
import type { AgentEvent } from "../../../app/src/shared/protocol.js";
import { writeBundleWithRustCli } from "./rust_bundle_cli.js";

export interface BundleBuilder {
  buildBundle(options: BuildBundleOptions): Promise<BuildBundleResult>;
}

type ProgressEmitter = (event: AgentEvent) => void;

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

/**
 * Delete any existing bundles for this repo+preset before writing new one
 */
async function cleanupExistingBundles(bundleDir: string, repoId: string, presetId: string): Promise<void> {
  const prefix = `${repoId}_${presetId}_`;
  try {
    const entries = await fs.readdir(bundleDir);
    for (const name of entries) {
      if (name.startsWith(prefix) && name.endsWith(".zip")) {
        try {
          await fs.unlink(path.posix.join(bundleDir, name));
        } catch {
          // Ignore deletion errors
        }
      }
    }
  } catch {
    // Directory may not exist yet
  }
}

export function createBundleBuilder(
  _pathConfig: PathBridgeConfig,
  emitProgress?: ProgressEmitter
): BundleBuilder {
  async function buildBundle(options: BuildBundleOptions): Promise<BuildBundleResult> {
    const { repoId, repoRoot, presetId, presetName, selection, outputDir } = options;
    const builtAt = new Date();
    const builtAtIso = builtAt.toISOString();

    const emitBundleProgress = (
      phase: "scanning" | "zipping" | "finalizing",
      filesDone: number,
      filesTotal: number
    ): void => {
      emitProgress?.({
        type: "bundleBuildProgress",
        repoId,
        presetId,
        phase,
        filesDone,
        filesTotal,
      });
    };

    // 1. Ensure output directory exists
    const bundleDir = path.posix.join(outputDir, "bundles", repoId, presetId);
    await fs.mkdir(bundleDir, { recursive: true });

    // 2. Get git info (best-effort)
    const gitInfo = await getGitInfo(repoRoot);

    // 3. Delete any existing bundles for this preset (only keep one)
    await cleanupExistingBundles(bundleDir, repoId, presetId);

    // 4. Generate timestamped filename (unique per build, recognizable)
    const timestamp = formatTimestamp(builtAt);
    const shortSha = gitInfo.shortSha;
    const baseName = `${repoId}_${presetId}_${timestamp}`;
    const fileName = shortSha ? `${baseName}_${shortSha}.zip` : `${baseName}.zip`;
    const wslPath = path.posix.join(bundleDir, fileName);

    // 5. Create temp file for atomic write
    const tempPath = wslPath + `.${crypto.randomUUID()}.tmp`;

    try {
      // 6. Scan + zip via Rust bundle CLI
      const git = {
        ...(gitInfo.headSha !== undefined ? { headSha: gitInfo.headSha } : {}),
        ...(gitInfo.shortSha !== undefined ? { shortSha: gitInfo.shortSha } : {}),
        ...(gitInfo.branch !== undefined ? { branch: gitInfo.branch } : {}),
      };

      const bundleResult = await writeBundleWithRustCli({
        outputPath: tempPath,
        repoRoot,
        repoId,
        presetId,
        presetName,
        selection: {
          includeRoot: selection.includeRoot,
          topLevelDirs: selection.topLevelDirs,
          excludedSubdirs: selection.excludedSubdirs ?? [],
        },
        git,
        builtAtIso,
        onProgress: (progress) => {
          emitBundleProgress(progress.phase, progress.filesDone, progress.filesTotal);
        },
      });

      emitBundleProgress("finalizing", bundleResult.fileCount, bundleResult.fileCount);

      // 9. Atomic rename
      await fs.rename(tempPath, wslPath);

      // 10. Convert path to Windows
      const windowsPath = wslToWindows(wslPath);

      if (bundleResult.scanMs !== undefined || bundleResult.zipMs !== undefined) {
        logger.info("Bundle timing", {
          repoId,
          presetId,
          scanMs: bundleResult.scanMs ?? null,
          zipMs: bundleResult.zipMs ?? null,
        });
      }

      logger.info("Bundle built", {
        repoId,
        presetId,
        fileCount: bundleResult.fileCount,
        bytes: bundleResult.bytes,
      });

      return {
        wslPath,
        windowsPath,
        aliasWslPath: wslPath,
        aliasWindowsPath: windowsPath,
        bytes: bundleResult.bytes,
        fileCount: bundleResult.fileCount,
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
