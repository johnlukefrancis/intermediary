// Path: app/src/shared/config.ts
// Description: AppConfig Zod schema and types

import { z } from "zod";
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
  GlobalExcludesSchema,
} from "./global_excludes.js";

export { GlobalExcludesSchema } from "./global_excludes.js";
export type { GlobalExcludes } from "./global_excludes.js";

// -----------------------------------------------------------------------------
// Bundle preset configuration
// -----------------------------------------------------------------------------

/**
 * Configuration for a bundle preset
 */
export const BundlePresetSchema = z.object({
  /** Unique identifier for this preset */
  presetId: z.string().min(1),
  /** Display name in the UI */
  presetName: z.string().min(1),
  /** Whether to include root-level files */
  includeRoot: z.boolean().default(true),
  /** Top-level directories to include (empty = default to all at runtime) */
  topLevelDirs: z.array(z.string().min(1)).default([]),
});

export type BundlePreset = z.infer<typeof BundlePresetSchema>;

export const DEFAULT_BUNDLE_PRESET: BundlePreset = {
  presetId: "context",
  presetName: "Context",
  includeRoot: true,
  topLevelDirs: [],
};

// -----------------------------------------------------------------------------
// Glob defaults
// -----------------------------------------------------------------------------

export const DEFAULT_DOCS_GLOBS = [
  "docs/**",
  "**/*.md",
  "**/*.mdx",
  "**/*.txt",
  "**/*.rst",
  "**/*.adoc",
  "**/*.asciidoc",
  "**/*.wiki",
  "**/README*",
];

export const DEFAULT_CODE_GLOBS = [
  "src/**",
  "app/**",
  "agent/**",
  "crates/**",
  "src-tauri/**",
  "**/*.ts",
  "**/*.tsx",
  "**/*.js",
  "**/*.jsx",
  "**/*.mjs",
  "**/*.cjs",
  "**/*.rs",
  "**/*.toml",
  "**/*.json",
  "**/*.yaml",
  "**/*.yml",
  "**/*.py",
  "**/*.go",
];

export const DEFAULT_IGNORE_GLOBS = [
  "**/node_modules/**",
  "**/.git/**",
  "**/dist/**",
  "**/build/**",
  "**/target/**",
  "**/.cache/**",
  "**/logs/**",
  "**/scripts/zip/output/**",
  "**/scripts/zip/Output/**",
  "**/Scripts/Zip/Output/**",
];

/**
 * Configuration for a single repository
 */
export const RepoConfigSchema = z.object({
  /** Unique identifier for this repo config */
  repoId: z.string(),
  /** Display name in the UI (shown in dropdown for grouped repos) */
  label: z.string(),
  /** Absolute WSL path to the repo root */
  wslPath: z.string(),
  /** Optional group ID - repos with same groupId share a tab with dropdown */
  groupId: z.string().optional(),
  /** Group display name (shown as tab label for grouped repos) */
  groupLabel: z.string().optional(),
  /** Whether to auto-stage file changes */
  autoStage: z.boolean().default(true),
  /** Globs that classify docs */
  docsGlobs: z.array(z.string()).default(DEFAULT_DOCS_GLOBS),
  /** Globs that classify code */
  codeGlobs: z.array(z.string()).default(DEFAULT_CODE_GLOBS),
  /** Globs that should be ignored by watchers */
  ignoreGlobs: z.array(z.string()).default(DEFAULT_IGNORE_GLOBS),
  /** Bundle presets for this repo */
  bundlePresets: z.array(BundlePresetSchema).default([DEFAULT_BUNDLE_PRESET]),
});

export type RepoConfig = z.infer<typeof RepoConfigSchema>;

/**
 * Global application configuration
 */
export const AppConfigSchema = z.object({
  /** Hostname for the agent WebSocket server */
  agentHost: z.string().min(1).default("localhost"),
  /** Port the agent WebSocket server listens on */
  agentPort: z.number().int().min(1024).max(65535).default(3141),
  /** Global default for auto-staging */
  autoStageGlobal: z.boolean().default(true),
  /** Configured repositories */
  repos: z.array(RepoConfigSchema).default([]),
});

export type AppConfig = z.infer<typeof AppConfigSchema>;

/**
 * Parse and validate config from unknown input
 */
export function parseAppConfig(input: unknown): AppConfig {
  return AppConfigSchema.parse(input);
}

/**
 * Get default config
 */
export function getDefaultConfig(): AppConfig {
  return DEFAULT_APP_CONFIG;
}

export const DEFAULT_APP_CONFIG: AppConfig = AppConfigSchema.parse({
  agentHost: "localhost",
  agentPort: 3141,
  autoStageGlobal: true,
  repos: [], // Empty by default - users add repos via the UI
});

// -----------------------------------------------------------------------------
// Persisted config (includes UI state and bundle selections)
// -----------------------------------------------------------------------------

/** Current config schema version */
export const CONFIG_VERSION = 8;

/** Remembered UI state */
export const UiStateSchema = z.object({
  /** Last active repo (by repoId) */
  lastActiveTabId: z.string().nullable().default(null),
});

export type UiState = z.infer<typeof UiStateSchema>;

/** Bundle selection state for a preset */
export const BundleSelectionSchema = z.object({
  includeRoot: z.boolean(),
  topLevelDirs: z.array(z.string()),
  excludedSubdirs: z.array(z.string()).default([]),
});

export type BundleSelection = z.infer<typeof BundleSelectionSchema>;

/** Bundle selections map: repoId -> presetId -> selection */
export const BundleSelectionsSchema = z.record(
  z.string(),
  z.record(z.string(), BundleSelectionSchema)
);

export type BundleSelections = z.infer<typeof BundleSelectionsSchema>;

/**
 * Full persisted configuration (saved to disk)
 */
export const PersistedConfigSchema = z.object({
  /** Schema version for migrations */
  configVersion: z.number().int().min(1).default(CONFIG_VERSION),
  /** Agent host */
  agentHost: z.string().min(1).default("localhost"),
  /** Agent port */
  agentPort: z.number().int().min(1024).max(65535).default(3141),
  /** Global auto-stage setting */
  autoStageGlobal: z.boolean().default(true),
  /** Configured repositories */
  repos: z.array(RepoConfigSchema).default([]),
  /** Remembered UI state */
  uiState: UiStateSchema.default({}),
  /** Bundle selections per repo/preset */
  bundleSelections: BundleSelectionsSchema.default({}),
  /** Global bundle excludes (extensions and patterns) */
  globalExcludes: GlobalExcludesSchema.default({
    dirNames: [...GLOBAL_EXCLUDE_RECOMMENDED_DIRS],
    dirSuffixes: [...GLOBAL_EXCLUDE_RECOMMENDED_DIR_SUFFIXES],
    fileNames: [...GLOBAL_EXCLUDE_RECOMMENDED_FILES],
    extensions: [
      ...GLOBAL_EXCLUDE_RECOMMENDED_EXTENSIONS,
      ...GLOBAL_EXCLUDE_RECOMMENDED_FILE_SUFFIXES,
    ],
    patterns: [...GLOBAL_EXCLUDE_RECOMMENDED_PATTERNS],
  }),
});

export type PersistedConfig = z.infer<typeof PersistedConfigSchema>;

/** Result from load_config command */
export interface LoadConfigResult {
  config: PersistedConfig;
  wasCreated: boolean;
  migrationApplied: boolean;
}

/**
 * Parse persisted config, applying migrations if needed
 */
export function parsePersistedConfig(input: unknown): PersistedConfig {
  const normalized = normalizeLegacyGlobalExcludes(input);
  const parsed = PersistedConfigSchema.parse(normalized);
  return migrateConfig(parsed);
}

/**
 * Get default persisted config
 */
export function getDefaultPersistedConfig(): PersistedConfig {
  return {
    configVersion: CONFIG_VERSION,
    ...DEFAULT_APP_CONFIG,
    uiState: {
      lastActiveTabId: null,
    },
    bundleSelections: {},
    globalExcludes: {
      dirNames: [...GLOBAL_EXCLUDE_RECOMMENDED_DIRS],
      dirSuffixes: [...GLOBAL_EXCLUDE_RECOMMENDED_DIR_SUFFIXES],
      fileNames: [...GLOBAL_EXCLUDE_RECOMMENDED_FILES],
      extensions: [
        ...GLOBAL_EXCLUDE_RECOMMENDED_EXTENSIONS,
        ...GLOBAL_EXCLUDE_RECOMMENDED_FILE_SUFFIXES,
      ],
      patterns: [...GLOBAL_EXCLUDE_RECOMMENDED_PATTERNS],
    },
  };
}

/**
 * Apply migrations to bring config to current version
 */
function migrateConfig(config: PersistedConfig): PersistedConfig {
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

  let next = { ...config };

  // Migration: v7 -> v8: Add new recommended binary/model-weight extensions.
  if (config.configVersion < 8) {
    next = migrateRecommendedExtensions(next);
  }

  return { ...next, configVersion: CONFIG_VERSION };
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

function normalizeLegacyGlobalExcludes(input: unknown): unknown {
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

/**
 * Extract AppConfig from PersistedConfig
 */
export function extractAppConfig(persisted: PersistedConfig): AppConfig {
  return {
    agentHost: persisted.agentHost,
    agentPort: persisted.agentPort,
    autoStageGlobal: persisted.autoStageGlobal,
    repos: persisted.repos,
  };
}
