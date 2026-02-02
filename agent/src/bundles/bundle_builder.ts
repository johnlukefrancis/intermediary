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
 * Format timestamp for bundle filename (UTC to match manifest's generatedAt)
 */
function formatTimestamp(date: Date): string {
  const y = date.getUTCFullYear();
  const mo = String(date.getUTCMonth() + 1).padStart(2, "0");
  const d = String(date.getUTCDate()).padStart(2, "0");
  const h = String(date.getUTCHours()).padStart(2, "0");
  const mi = String(date.getUTCMinutes()).padStart(2, "0");
  const s = String(date.getUTCSeconds()).padStart(2, "0");
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

    const emitBundleProgress = (payload: {
      phase: "scanning" | "zipping" | "finalizing";
      filesDone: number;
      filesTotal: number;
      currentFile?: string;
      currentBytesDone?: number;
      currentBytesTotal?: number;
      bytesDoneTotalBestEffort?: number;
    }): void => {
      emitProgress?.({
        type: "bundleBuildProgress",
        repoId,
        presetId,
        phase: payload.phase,
        filesDone: payload.filesDone,
        filesTotal: payload.filesTotal,
        ...(payload.currentFile !== undefined ? { currentFile: payload.currentFile } : {}),
        ...(payload.currentBytesDone !== undefined ? { currentBytesDone: payload.currentBytesDone } : {}),
        ...(payload.currentBytesTotal !== undefined ? { currentBytesTotal: payload.currentBytesTotal } : {}),
        ...(payload.bytesDoneTotalBestEffort !== undefined
          ? { bytesDoneTotalBestEffort: payload.bytesDoneTotalBestEffort }
          : {}),
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
        ...(options.globalExcludes ? { globalExcludes: options.globalExcludes } : {}),
        onProgress: (progress) => {
          const payload: {
            phase: "scanning" | "zipping";
            filesDone: number;
            filesTotal: number;
            currentFile?: string;
            currentBytesDone?: number;
            currentBytesTotal?: number;
            bytesDoneTotalBestEffort?: number;
          } = {
            phase: progress.phase,
            filesDone: progress.filesDone,
            filesTotal: progress.filesTotal,
          };
          if (progress.currentFile !== undefined) {
            payload.currentFile = progress.currentFile;
          }
          if (progress.currentBytesDone !== undefined) {
            payload.currentBytesDone = progress.currentBytesDone;
          }
          if (progress.currentBytesTotal !== undefined) {
            payload.currentBytesTotal = progress.currentBytesTotal;
          }
          if (progress.bytesDoneTotalBestEffort !== undefined) {
            payload.bytesDoneTotalBestEffort = progress.bytesDoneTotalBestEffort;
          }
          emitBundleProgress(payload);
        },
      });

      emitBundleProgress({
        phase: "finalizing",
        filesDone: bundleResult.fileCount,
        filesTotal: bundleResult.fileCount,
      });

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
