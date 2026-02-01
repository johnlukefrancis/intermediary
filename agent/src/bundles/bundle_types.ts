// Path: agent/src/bundles/bundle_types.ts
// Description: Type definitions for bundle building

/**
 * Manifest embedded in each generated bundle zip
 */
export interface BundleManifest {
  generatedAt: string;
  repoId: string;
  repoRoot: string;
  presetId: string;
  presetName: string;
  selection: {
    includeRoot: boolean;
    topLevelDirsIncluded: string[];
    excludedSubdirs: string[];
  };
  git: {
    headSha?: string;
    shortSha?: string;
    branch?: string;
  };
  fileCount: number;
  totalBytesBestEffort: number;
}

/**
 * Global excludes for bundle building
 */
export interface GlobalExcludes {
  dirNames: string[];
  dirSuffixes: string[];
  fileNames: string[];
  extensions: string[];
  patterns: string[];
}

/**
 * Options for building a bundle
 */
export interface BuildBundleOptions {
  repoId: string;
  repoRoot: string;
  presetId: string;
  presetName: string;
  selection: {
    includeRoot: boolean;
    topLevelDirs: string[];
    excludedSubdirs?: string[];
  };
  outputDir: string;
  /** User-configurable global excludes */
  globalExcludes?: GlobalExcludes;
}

/**
 * Result of a successful bundle build
 */
export interface BuildBundleResult {
  wslPath: string;
  windowsPath: string;
  aliasWslPath: string;
  aliasWindowsPath: string;
  bytes: number;
  fileCount: number;
  builtAtIso: string;
}
