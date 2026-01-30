// Path: agent/src/staging/stager.ts
// Description: Atomic file copy with debounced auto-staging

import * as fs from "node:fs/promises";
import * as path from "node:path";
import * as crypto from "node:crypto";
import { type PathBridgeConfig, buildStagedPaths } from "./path_bridge.js";
import { logger } from "../util/logger.js";
import { AgentError } from "../util/errors.js";

const AUTO_STAGE_DEBOUNCE_MS = 250;

export interface StageResult {
  wslPath: string;
  windowsPath: string;
  bytesCopied: number;
  mtimeMs: number;
}

export interface Stager {
  stageFile(repoId: string, repoRoot: string, relativePath: string): Promise<StageResult>;
  scheduleAutoStage(
    repoId: string,
    repoRoot: string,
    relativePath: string,
    onSuccess: (result: StageResult) => void,
    onError?: (err: unknown) => void
  ): void;
  cancelPendingStage(repoId: string, relativePath: string): void;
}

/**
 * Validate that a relative path is safe (no traversal, no absolute).
 */
function validateRelativePath(relativePath: string): void {
  if (!relativePath) {
    throw new AgentError("INVALID_PATH", "Empty relative paths not allowed");
  }
  if (path.posix.isAbsolute(relativePath) || path.win32.isAbsolute(relativePath)) {
    throw new AgentError("INVALID_PATH", "Absolute paths not allowed");
  }
  if (relativePath.includes("\\")) {
    throw new AgentError("INVALID_PATH", "Backslashes not allowed in relative paths");
  }
  const normalized = path.posix.normalize(relativePath);
  if (normalized === "." || normalized.startsWith("..")) {
    throw new AgentError("INVALID_PATH", "Path traversal not allowed");
  }
}

export function createStager(config: PathBridgeConfig): Stager {
  const pendingStages = new Map<string, NodeJS.Timeout>();

  function getPendingKey(repoId: string, relativePath: string): string {
    return `${repoId}:${relativePath}`;
  }

  async function stageFile(
    repoId: string,
    repoRoot: string,
    relativePath: string
  ): Promise<StageResult> {
    validateRelativePath(relativePath);

    const sourcePath = path.join(repoRoot, relativePath);
    const { wslPath, windowsPath } = buildStagedPaths(config, repoId, relativePath);

    // Ensure destination directory exists
    const destDir = path.dirname(wslPath);
    await fs.mkdir(destDir, { recursive: true });

    // Create temp file for atomic write
    const tempSuffix = `.${crypto.randomUUID()}.tmp`;
    const tempPath = wslPath + tempSuffix;

    try {
      // Copy to temp file
      await fs.copyFile(sourcePath, tempPath);

      // Atomic rename
      await fs.rename(tempPath, wslPath);

      // Get file stats
      const stat = await fs.stat(wslPath);

      logger.debug("File staged", { repoId, relativePath, wslPath });

      return {
        wslPath,
        windowsPath,
        bytesCopied: stat.size,
        mtimeMs: stat.mtimeMs,
      };
    } catch (err) {
      // Clean up temp file on failure
      try {
        await fs.unlink(tempPath);
      } catch {
        // Ignore cleanup errors
      }
      throw err;
    }
  }

  function scheduleAutoStage(
    repoId: string,
    repoRoot: string,
    relativePath: string,
    onSuccess: (result: StageResult) => void,
    onError?: (err: unknown) => void
  ): void {
    const key = getPendingKey(repoId, relativePath);

    // Cancel any existing pending stage for this file
    const existing = pendingStages.get(key);
    if (existing) {
      clearTimeout(existing);
    }

    // Schedule new debounced stage
    const timeout = setTimeout(() => {
      pendingStages.delete(key);
      stageFile(repoId, repoRoot, relativePath)
        .then(onSuccess)
        .catch((err: unknown) => {
          logger.error("Auto-stage failed", {
            repoId,
            relativePath,
            error: err instanceof Error ? err.message : String(err),
          });
          onError?.(err);
        });
    }, AUTO_STAGE_DEBOUNCE_MS);

    pendingStages.set(key, timeout);
  }

  function cancelPendingStage(repoId: string, relativePath: string): void {
    const key = getPendingKey(repoId, relativePath);
    const existing = pendingStages.get(key);
    if (existing) {
      clearTimeout(existing);
      pendingStages.delete(key);
    }
  }

  return {
    stageFile,
    scheduleAutoStage,
    cancelPendingStage,
  };
}
