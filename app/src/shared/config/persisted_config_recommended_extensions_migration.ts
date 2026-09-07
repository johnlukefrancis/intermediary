// Path: app/src/shared/config/persisted_config_recommended_extensions_migration.ts
// Description: Merge newly recommended exclude extensions into configs that still carry the recommended baseline

import {
  GLOBAL_EXCLUDE_EXTENSION_ADDITIONS_BY_VERSION,
  GLOBAL_EXCLUDE_RECOMMENDED_DIRS,
  GLOBAL_EXCLUDE_RECOMMENDED_DIR_SUFFIXES,
  GLOBAL_EXCLUDE_RECOMMENDED_EXTENSIONS,
  GLOBAL_EXCLUDE_RECOMMENDED_FILE_SUFFIXES,
  GLOBAL_EXCLUDE_RECOMMENDED_FILES,
  GLOBAL_EXCLUDE_RECOMMENDED_PATTERNS,
} from "../global_excludes.js";
import {
  mergeUnique,
  normalizeExtensionValue,
  normalizeNameValue,
  normalizePatternValue,
} from "./persisted_config_global_excludes_migration.js";
import type { PersistedConfig } from "./persisted_config.js";

/**
 * Apply the recommended-extension addition introduced at `version`.
 *
 * A `seed: "all"` addition lands in every config once: the user never had
 * these entries, so seeding them overrides no decision, and removing them
 * afterwards sticks. A `seed: "baseline"` addition only reaches configs that
 * still carry every recommended entry as it stood before that version.
 */
export function migrateRecommendedExtensionAdditions(
  config: PersistedConfig,
  version: number
): PersistedConfig {
  const step = GLOBAL_EXCLUDE_EXTENSION_ADDITIONS_BY_VERSION.find(
    (entry) => entry.version === version
  );
  if (!step) {
    return config;
  }

  const addedHere = step.extensions.map(normalizeExtensionValue);
  const addedAtOrAfter = new Set(
    GLOBAL_EXCLUDE_EXTENSION_ADDITIONS_BY_VERSION.filter(
      (entry) => entry.version >= version
    )
      .flatMap((entry) => entry.extensions)
      .map(normalizeExtensionValue)
  );
  const baselineExtensions = GLOBAL_EXCLUDE_RECOMMENDED_EXTENSIONS.map(
    normalizeExtensionValue
  ).filter((value) => !addedAtOrAfter.has(value));

  if (
    step.seed === "baseline" &&
    !carriesRecommendedBaseline(config, baselineExtensions)
  ) {
    return config;
  }

  return {
    ...config,
    globalExcludes: {
      ...config.globalExcludes,
      extensions: mergeUnique(config.globalExcludes.extensions, addedHere),
    },
  };
}

function carriesRecommendedBaseline(
  config: PersistedConfig,
  baselineExtensions: string[]
): boolean {
  const extensionSet = normalizedSet(
    config.globalExcludes.extensions,
    normalizeExtensionValue
  );
  const patternSet = normalizedSet(
    config.globalExcludes.patterns,
    normalizePatternValue
  );
  const dirNameSet = normalizedSet(
    config.globalExcludes.dirNames,
    normalizePatternValue
  );
  const dirSuffixSet = normalizedSet(
    config.globalExcludes.dirSuffixes,
    normalizeExtensionValue
  );
  const fileNameSet = normalizedSet(
    config.globalExcludes.fileNames,
    normalizeNameValue
  );

  return (
    baselineExtensions.every((ext) => extensionSet.has(ext)) &&
    GLOBAL_EXCLUDE_RECOMMENDED_FILE_SUFFIXES.map(normalizeExtensionValue).every(
      (suffix) => extensionSet.has(suffix)
    ) &&
    GLOBAL_EXCLUDE_RECOMMENDED_PATTERNS.map(normalizePatternValue).every(
      (pattern) => patternSet.has(pattern)
    ) &&
    GLOBAL_EXCLUDE_RECOMMENDED_DIRS.map(normalizePatternValue).every((name) =>
      dirNameSet.has(name)
    ) &&
    GLOBAL_EXCLUDE_RECOMMENDED_DIR_SUFFIXES.map(normalizeExtensionValue).every(
      (suffix) => dirSuffixSet.has(suffix)
    ) &&
    GLOBAL_EXCLUDE_RECOMMENDED_FILES.map(normalizeNameValue).every((name) =>
      fileNameSet.has(name)
    )
  );
}

function normalizedSet(
  values: string[],
  normalize: (value: string) => string
): Set<string> {
  return new Set(values.map(normalize).filter((value) => value.length > 0));
}
