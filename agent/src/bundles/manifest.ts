// Path: agent/src/bundles/manifest.ts
// Description: Manifest generation for bundle zips

import type { BundleManifest } from "./bundle_types.js";
import type { GitInfo } from "./git_info.js";

/**
 * Create a manifest object for a bundle
 */
export function createManifest(
  repoId: string,
  repoRoot: string,
  presetId: string,
  presetName: string,
  selection: { includeRoot: boolean; topLevelDirsIncluded: string[] },
  git: GitInfo,
  fileCount: number,
  totalBytesBestEffort: number
): BundleManifest {
  return {
    generatedAt: new Date().toISOString(),
    repoId,
    repoRoot,
    presetId,
    presetName,
    selection,
    git,
    fileCount,
    totalBytesBestEffort,
  };
}

/**
 * Serialize manifest to pretty-printed JSON
 */
export function serializeManifest(manifest: BundleManifest): string {
  return JSON.stringify(manifest, null, 2);
}
