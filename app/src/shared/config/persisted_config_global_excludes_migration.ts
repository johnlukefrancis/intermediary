// Path: app/src/shared/config/persisted_config_global_excludes_migration.ts
// Description: Legacy global excludes normalization and the model-dir pattern migration

import {
  GLOBAL_EXCLUDE_RECOMMENDED_DIRS,
  GLOBAL_EXCLUDE_RECOMMENDED_DIR_SUFFIXES,
  GLOBAL_EXCLUDE_RECOMMENDED_EXTENSIONS,
  GLOBAL_EXCLUDE_RECOMMENDED_FILE_SUFFIXES,
  GLOBAL_EXCLUDE_RECOMMENDED_FILES,
  GLOBAL_EXCLUDE_RECOMMENDED_PATTERNS,
  GLOBAL_EXCLUDE_MODEL_WEIGHT_EXTENSIONS,
  GLOBAL_EXCLUDE_MODEL_FORMAT_EXTENSIONS,
  GLOBAL_EXCLUDE_MODEL_DIR_PATTERNS,
  GLOBAL_EXCLUDE_OPTIONAL_MODEL_DIR_PATTERNS,
  GLOBAL_EXCLUDE_HF_CACHE_PATTERNS,
  GLOBAL_EXCLUDE_EXPERIMENT_PATTERNS,
} from "../global_excludes.js";
import type { PersistedConfig } from "./persisted_config.js";

export function migrateLegacyModelDirPatterns(
  config: PersistedConfig
): PersistedConfig {
  const modelDirPatternSet = new Set(
    GLOBAL_EXCLUDE_OPTIONAL_MODEL_DIR_PATTERNS.map(normalizePatternValue)
  );
  const legacyRecommendedPatternSet = new Set(
    [
      ...GLOBAL_EXCLUDE_OPTIONAL_MODEL_DIR_PATTERNS,
      ...GLOBAL_EXCLUDE_HF_CACHE_PATTERNS,
      ...GLOBAL_EXCLUDE_EXPERIMENT_PATTERNS,
    ].map(normalizePatternValue)
  );
  const currentPatternSet = new Set(
    config.globalExcludes.patterns
      .map(normalizePatternValue)
      .filter((value) => value.length > 0)
  );

  if (!setsEqual(currentPatternSet, legacyRecommendedPatternSet)) {
    return config;
  }

  const filteredPatterns = config.globalExcludes.patterns.filter(
    (value) => !modelDirPatternSet.has(normalizePatternValue(value))
  );

  return {
    ...config,
    globalExcludes: {
      ...config.globalExcludes,
      patterns: filteredPatterns,
    },
  };
}

/**
 * v7 introduced explicit dir/file exclude lists. From that version on the
 * stored lists are the user's authority and the recommended set is never
 * re-merged; only pre-v7 preset-era configs are normalized here.
 */
const EXPLICIT_GLOBAL_EXCLUDES_VERSION = 7;

export function normalizeLegacyGlobalExcludes(input: unknown): unknown {
  if (!input || typeof input !== "object") return input;
  const raw = input as Record<string, unknown>;
  const configVersion =
    typeof raw.configVersion === "number" ? raw.configVersion : null;
  if (
    configVersion !== null &&
    configVersion >= EXPLICIT_GLOBAL_EXCLUDES_VERSION
  ) {
    return input;
  }
  const globalExcludes = raw.globalExcludes;
  if (!globalExcludes || typeof globalExcludes !== "object") {
    return input;
  }
  const rawExcludes = globalExcludes as Record<string, unknown>;
  const presets =
    rawExcludes.presets && typeof rawExcludes.presets === "object"
      ? (rawExcludes.presets as Record<string, unknown>)
      : null;

  const presetFlags = {
    modelWeights: presets?.modelWeights === true,
    modelFormats: presets?.modelFormats === true,
    modelDirs: presets?.modelDirs === true,
    hfCaches: presets?.hfCaches === true,
    experimentLogs: presets?.experimentLogs === true,
  };
  const hasExplicitPresetFlags = presets
    ? [
        "modelWeights",
        "modelFormats",
        "modelDirs",
        "hfCaches",
        "experimentLogs",
      ].some((key) => key in presets)
    : false;
  const legacyMlArtifacts = presets?.mlArtifacts;
  const useLegacyPresets =
    hasExplicitPresetFlags || typeof legacyMlArtifacts === "boolean";

  const existingDirNames = Array.isArray(rawExcludes.dirNames)
    ? rawExcludes.dirNames.filter((value): value is string => typeof value === "string")
    : [];
  const existingDirSuffixes = Array.isArray(rawExcludes.dirSuffixes)
    ? rawExcludes.dirSuffixes.filter((value): value is string => typeof value === "string")
    : [];
  const existingFileNames = Array.isArray(rawExcludes.fileNames)
    ? rawExcludes.fileNames.filter((value): value is string => typeof value === "string")
    : [];
  const existingExtensions = Array.isArray(rawExcludes.extensions)
    ? rawExcludes.extensions.filter((value): value is string => typeof value === "string")
    : [];
  const existingPatterns = Array.isArray(rawExcludes.patterns)
    ? rawExcludes.patterns.filter((value): value is string => typeof value === "string")
    : [];

  const mergedDirNames = mergeUnique(
    GLOBAL_EXCLUDE_RECOMMENDED_DIRS,
    existingDirNames
  );
  const mergedDirSuffixes = mergeUnique(
    GLOBAL_EXCLUDE_RECOMMENDED_DIR_SUFFIXES,
    existingDirSuffixes
  );
  const mergedFileNames = mergeUnique(
    GLOBAL_EXCLUDE_RECOMMENDED_FILES,
    existingFileNames
  );

  const mlExtensions: string[] = [];
  const mlPatterns: string[] = [];
  if (useLegacyPresets) {
    const allMlEnabled =
      typeof legacyMlArtifacts === "boolean" ? legacyMlArtifacts : true;
    const modelWeights = hasExplicitPresetFlags ? presetFlags.modelWeights : allMlEnabled;
    const modelFormats = hasExplicitPresetFlags ? presetFlags.modelFormats : allMlEnabled;
    const modelDirs = hasExplicitPresetFlags ? presetFlags.modelDirs : allMlEnabled;
    const hfCaches = hasExplicitPresetFlags ? presetFlags.hfCaches : allMlEnabled;
    const experimentLogs = hasExplicitPresetFlags
      ? presetFlags.experimentLogs
      : allMlEnabled;
    if (modelWeights) mlExtensions.push(...GLOBAL_EXCLUDE_MODEL_WEIGHT_EXTENSIONS);
    if (modelFormats) mlExtensions.push(...GLOBAL_EXCLUDE_MODEL_FORMAT_EXTENSIONS);
    if (modelDirs) mlPatterns.push(...GLOBAL_EXCLUDE_MODEL_DIR_PATTERNS);
    if (hfCaches) mlPatterns.push(...GLOBAL_EXCLUDE_HF_CACHE_PATTERNS);
    if (experimentLogs) mlPatterns.push(...GLOBAL_EXCLUDE_EXPERIMENT_PATTERNS);
  } else {
    mlExtensions.push(...GLOBAL_EXCLUDE_RECOMMENDED_EXTENSIONS);
    mlPatterns.push(...GLOBAL_EXCLUDE_RECOMMENDED_PATTERNS);
  }

  const mergedExtensions = mergeUnique(
    GLOBAL_EXCLUDE_RECOMMENDED_FILE_SUFFIXES,
    existingExtensions,
    mlExtensions
  );
  const mergedPatterns = mergeUnique(existingPatterns, mlPatterns);

  return {
    ...raw,
    globalExcludes: {
      ...rawExcludes,
      dirNames: mergedDirNames,
      dirSuffixes: mergedDirSuffixes,
      fileNames: mergedFileNames,
      extensions: mergedExtensions,
      patterns: mergedPatterns,
    },
  };
}

export function normalizeExtensionValue(value: string): string {
  const trimmed = value.trim().toLowerCase();
  if (trimmed.length === 0) return "";
  if (trimmed === "~") return "~";
  return trimmed.startsWith(".") ? trimmed : `.${trimmed}`;
}

export function normalizePatternValue(value: string): string {
  return value.trim().replace(/^\/+|\/+$/g, "").toLowerCase();
}

export function normalizeNameValue(value: string): string {
  return value.trim().toLowerCase();
}

export function mergeUnique(...groups: string[][]): string[] {
  const merged = new Set<string>();
  groups.forEach((group) => {
    group.forEach((value) => merged.add(value));
  });
  return Array.from(merged);
}

function setsEqual(left: Set<string>, right: Set<string>): boolean {
  if (left.size !== right.size) {
    return false;
  }
  for (const value of left) {
    if (!right.has(value)) {
      return false;
    }
  }
  return true;
}
