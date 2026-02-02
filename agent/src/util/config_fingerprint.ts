// Path: agent/src/util/config_fingerprint.ts
// Description: Computes stable fingerprints for watcher-relevant config to detect changes

import type { AppConfig } from "../../../app/src/shared/config.js";

export interface WatcherConfigInput {
  config: AppConfig;
  stagingWslRoot: string;
  autoStageOnChange: boolean;
}

interface RepoFingerprint {
  repoId: string;
  wslPath: string;
  docsGlobs: string[];
  codeGlobs: string[];
  ignoreGlobs: string[];
  autoStage: boolean;
}

interface FingerprintData {
  stagingWslRoot: string;
  autoStageGlobal: boolean;
  repos: RepoFingerprint[];
}

/**
 * Compute a stable fingerprint of watcher-relevant config.
 * Returns a string that changes when watchers need to be reset.
 */
export function computeConfigFingerprint(input: WatcherConfigInput): string {
  const repos: RepoFingerprint[] = input.config.repos
    .map((repo) => ({
      repoId: repo.repoId,
      wslPath: repo.wslPath,
      docsGlobs: [...repo.docsGlobs].sort(),
      codeGlobs: [...repo.codeGlobs].sort(),
      ignoreGlobs: [...repo.ignoreGlobs].sort(),
      autoStage: repo.autoStage,
    }))
    .sort((a, b) => a.repoId.localeCompare(b.repoId));

  const data: FingerprintData = {
    stagingWslRoot: input.stagingWslRoot,
    autoStageGlobal: input.autoStageOnChange,
    repos,
  };

  return JSON.stringify(data);
}

/**
 * Check if two fingerprints indicate config has changed.
 */
export function configChanged(oldFp: string | null, newFp: string): boolean {
  return oldFp === null || oldFp !== newFp;
}
