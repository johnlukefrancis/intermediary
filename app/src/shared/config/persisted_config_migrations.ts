// Path: app/src/shared/config/persisted_config_migrations.ts
// Description: Persisted config migrations and legacy normalization

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
  GLOBAL_EXCLUDE_BINARY_EXTENSIONS,
  GLOBAL_EXCLUDE_HF_CACHE_PATTERNS,
  GLOBAL_EXCLUDE_EXPERIMENT_PATTERNS,
} from "../global_excludes.js";
import { CONFIG_VERSION } from "./version.js";
import type { PersistedConfig } from "./persisted_config.js";

/**
 * Apply migrations to bring config to current version
 */
export function migrateConfig(config: PersistedConfig): PersistedConfig {
  // Migration: v1 -> v2: Add excludedSubdirs to bundle selections
  // Zod schema's .default([]) already ensures excludedSubdirs exists after parsing.

  // Migration: v2 -> v3: Remove worktree fields, use repoId for tab identity
  // Old lastActiveTabId values (e.g. "texture-portal") won't match repoIds,
  // so app.tsx validation will gracefully fall back to first repo.

  // Migration: v3 -> v4: Add globalExcludes
  // Zod schema defaults handle missing globalExcludes fields.

  // Migration: v4 -> v5: Add globalExcludes presets (deprecated in v6).
  // Migration: v5 -> v6: Drop presets in favor of explicit extension/pattern lists.
  // Migration: v6 -> v7: Add explicit dir/file exclude lists.
  // Migration: v9 -> v10: Add per-tab texture id (optional).

  let next = { ...config };

  // Migration: v7 -> v8: Add new recommended binary/model-weight extensions.
  if (config.configVersion < 8) {
    next = migrateRecommendedExtensions(next);
  }

  // Migration: v11 -> v12: Normalize localhost agent host to loopback IP.
  if (config.configVersion < 12) {
    next = migrateLoopbackAgentHost(next);
  }
  // Migration: v12 -> v13: Add agent auto-start + distro override fields.
  if (config.configVersion < 13) {
    next = migrateAgentDefaults(next);
  }
  // Migration: v14 -> v15: Add uiState.lastActiveGroupRepoIds.

  return { ...next, configVersion: CONFIG_VERSION };
}

function migrateLoopbackAgentHost(config: PersistedConfig): PersistedConfig {
  if (config.agentHost !== "localhost") {
    return config;
  }

  return {
    ...config,
    agentHost: "127.0.0.1",
  };
}

function migrateRecommendedExtensions(config: PersistedConfig): PersistedConfig {
  const extensionSet = new Set(
    config.globalExcludes.extensions
      .map(normalizeExtensionValue)
      .filter((value) => value.length > 0)
  );
  const patternSet = new Set(
    config.globalExcludes.patterns
      .map(normalizePatternValue)
      .filter((value) => value.length > 0)
  );
  const dirNameSet = new Set(
    config.globalExcludes.dirNames
      .map(normalizePatternValue)
      .filter((value) => value.length > 0)
  );
  const dirSuffixSet = new Set(
    config.globalExcludes.dirSuffixes
      .map(normalizeExtensionValue)
      .filter((value) => value.length > 0)
  );
  const fileNameSet = new Set(
    config.globalExcludes.fileNames
      .map(normalizeNameValue)
      .filter((value) => value.length > 0)
  );

  const newRecommendedExtensions = [
    ...GLOBAL_EXCLUDE_BINARY_EXTENSIONS,
    ".gguf",
  ].map(normalizeExtensionValue);
  const legacyRecommendedExtensions = GLOBAL_EXCLUDE_RECOMMENDED_EXTENSIONS.filter(
    (value) => !newRecommendedExtensions.includes(normalizeExtensionValue(value))
  ).map(normalizeExtensionValue);
  const hasAllLegacyExtensions = legacyRecommendedExtensions.every((ext) =>
    extensionSet.has(ext)
  );
  const hasAllFileSuffixes = GLOBAL_EXCLUDE_RECOMMENDED_FILE_SUFFIXES.map(
    normalizeExtensionValue
  ).every((suffix) => extensionSet.has(suffix));
  const hasAllPatterns = GLOBAL_EXCLUDE_RECOMMENDED_PATTERNS.map(
    normalizePatternValue
  ).every((pattern) => patternSet.has(pattern));
  const hasAllDirNames = GLOBAL_EXCLUDE_RECOMMENDED_DIRS.map(normalizePatternValue).every(
    (name) => dirNameSet.has(name)
  );
  const hasAllDirSuffixes = GLOBAL_EXCLUDE_RECOMMENDED_DIR_SUFFIXES.map(
    normalizeExtensionValue
  ).every((suffix) => dirSuffixSet.has(suffix));
  const hasAllFileNames = GLOBAL_EXCLUDE_RECOMMENDED_FILES.map(normalizeNameValue).every(
    (name) => fileNameSet.has(name)
  );

  const looksLikeRecommended =
    hasAllLegacyExtensions &&
    hasAllFileSuffixes &&
    hasAllPatterns &&
    hasAllDirNames &&
    hasAllDirSuffixes &&
    hasAllFileNames;

  if (!looksLikeRecommended) {
    return config;
  }

  const mergedExtensions = mergeUnique(
    config.globalExcludes.extensions,
    newRecommendedExtensions
  );

  return {
    ...config,
    globalExcludes: {
      ...config.globalExcludes,
      extensions: mergedExtensions,
    },
  };
}

function migrateAgentDefaults(config: PersistedConfig): PersistedConfig {
  const trimmedDistro = config.agentDistro?.trim() ?? "";
  return {
    ...config,
    agentAutoStart: config.agentAutoStart,
    agentDistro: trimmedDistro.length > 0 ? trimmedDistro : null,
  };
}

export function normalizeLegacyGlobalExcludes(input: unknown): unknown {
  if (!input || typeof input !== "object") return input;
  const raw = input as Record<string, unknown>;
  const configVersion =
    typeof raw.configVersion === "number" ? raw.configVersion : null;
  if (configVersion !== null && configVersion >= CONFIG_VERSION) {
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

function normalizeExtensionValue(value: string): string {
  const trimmed = value.trim().toLowerCase();
  if (trimmed.length === 0) return "";
  if (trimmed === "~") return "~";
  return trimmed.startsWith(".") ? trimmed : `.${trimmed}`;
}

function normalizePatternValue(value: string): string {
  return value.trim().replace(/^\/+|\/+$/g, "").toLowerCase();
}

function normalizeNameValue(value: string): string {
  return value.trim().toLowerCase();
}

function mergeUnique(...groups: string[][]): string[] {
  const merged = new Set<string>();
  groups.forEach((group) => {
    group.forEach((value) => merged.add(value));
  });
  return Array.from(merged);
}
