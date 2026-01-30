// Path: agent/src/bundles/git_info.ts
// Description: Best-effort git info extraction for bundle manifests

import { exec } from "node:child_process";
import { promisify } from "node:util";
import { logger } from "../util/logger.js";

const execAsync = promisify(exec);

export interface GitInfo {
  headSha?: string;
  shortSha?: string;
  branch?: string;
}

/**
 * Extract git info from a repository (best-effort, never throws)
 */
export async function getGitInfo(repoRoot: string): Promise<GitInfo> {
  const result: GitInfo = {};

  try {
    const { stdout: sha } = await execAsync("git rev-parse HEAD", {
      cwd: repoRoot,
      timeout: 5000,
    });
    result.headSha = sha.trim();
    result.shortSha = result.headSha.slice(0, 7);
  } catch {
    logger.debug("Could not get git HEAD SHA", { repoRoot });
  }

  try {
    const { stdout: branch } = await execAsync("git rev-parse --abbrev-ref HEAD", {
      cwd: repoRoot,
      timeout: 5000,
    });
    result.branch = branch.trim();
  } catch {
    logger.debug("Could not get git branch", { repoRoot });
  }

  return result;
}
