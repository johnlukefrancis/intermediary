// Path: app/src/components/options/excludes/excludes_recommendations.ts
// Description: Helpers for recommended global excludes toggles

import {
  GLOBAL_EXCLUDE_RECOMMENDED_DIRS,
  GLOBAL_EXCLUDE_RECOMMENDED_DIR_SUFFIXES,
  GLOBAL_EXCLUDE_RECOMMENDED_EXTENSIONS,
  GLOBAL_EXCLUDE_RECOMMENDED_FILE_SUFFIXES,
  GLOBAL_EXCLUDE_RECOMMENDED_FILES,
  GLOBAL_EXCLUDE_RECOMMENDED_PATTERNS,
} from "../../../shared/global_excludes.js";

import type { GlobalExcludes } from "../../../shared/global_excludes.js";

interface RecommendedSets {
  extensions: Set<string>;
  patterns: Set<string>;
  dirNames: Set<string>;
  dirSuffixes: Set<string>;
  fileNames: Set<string>;
}

export function isRecommendedEnabled(sets: RecommendedSets): boolean {
  const hasAllExtensions = GLOBAL_EXCLUDE_RECOMMENDED_EXTENSIONS.every((ext) =>
    sets.extensions.has(ext)
  );
  const hasAllFileSuffixes = GLOBAL_EXCLUDE_RECOMMENDED_FILE_SUFFIXES.every((ext) =>
    sets.extensions.has(ext)
  );
  const hasAllPatterns = GLOBAL_EXCLUDE_RECOMMENDED_PATTERNS.every((pattern) =>
    sets.patterns.has(pattern)
  );
  const hasAllDirNames = GLOBAL_EXCLUDE_RECOMMENDED_DIRS.every((name) =>
    sets.dirNames.has(name)
  );
  const hasAllDirSuffixes = GLOBAL_EXCLUDE_RECOMMENDED_DIR_SUFFIXES.every((suffix) =>
    sets.dirSuffixes.has(suffix)
  );
  const hasAllFileNames = GLOBAL_EXCLUDE_RECOMMENDED_FILES.every((name) =>
    sets.fileNames.has(name)
  );

  return (
    hasAllExtensions &&
    hasAllFileSuffixes &&
    hasAllPatterns &&
    hasAllDirNames &&
    hasAllDirSuffixes &&
    hasAllFileNames
  );
}

export function buildRecommendedExcludes(enabled: boolean): GlobalExcludes {
  return {
    dirNames: enabled ? [...GLOBAL_EXCLUDE_RECOMMENDED_DIRS] : [],
    dirSuffixes: enabled ? [...GLOBAL_EXCLUDE_RECOMMENDED_DIR_SUFFIXES] : [],
    fileNames: enabled ? [...GLOBAL_EXCLUDE_RECOMMENDED_FILES] : [],
    extensions: enabled
      ? [...GLOBAL_EXCLUDE_RECOMMENDED_EXTENSIONS, ...GLOBAL_EXCLUDE_RECOMMENDED_FILE_SUFFIXES]
      : [],
    patterns: enabled ? [...GLOBAL_EXCLUDE_RECOMMENDED_PATTERNS] : [],
  };
}
