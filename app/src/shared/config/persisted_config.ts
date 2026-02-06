// Path: app/src/shared/config/persisted_config.ts
// Description: Persisted config schema, types, and defaults

import { z } from "zod";
import { GlobalExcludesSchema } from "../global_excludes.js";
import {
  GLOBAL_EXCLUDE_RECOMMENDED_DIRS,
  GLOBAL_EXCLUDE_RECOMMENDED_DIR_SUFFIXES,
  GLOBAL_EXCLUDE_RECOMMENDED_EXTENSIONS,
  GLOBAL_EXCLUDE_RECOMMENDED_FILE_SUFFIXES,
  GLOBAL_EXCLUDE_RECOMMENDED_FILES,
  GLOBAL_EXCLUDE_RECOMMENDED_PATTERNS,
} from "../global_excludes.js";
import { DEFAULT_APP_CONFIG } from "./app_config.js";
import { RepoConfigSchema } from "./repo_config.js";
import { CONFIG_VERSION } from "./version.js";
import {
  migrateConfig,
  normalizeLegacyCodeGlobs,
  normalizeLegacyGlobalExcludes,
  normalizeLegacyRepoRoots,
} from "./persisted_config_migrations.js";

/** Remembered UI state */
export const UiStateSchema = z.object({
  /** Last active repo (by repoId) */
  lastActiveTabId: z.string().nullable().default(null),
  /** Last active repo per group (groupId -> repoId) */
  lastActiveGroupRepoIds: z.record(z.string(), z.string()).default({}),
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

/** Per-tab theme configuration */
export const TabThemeSchema = z.object({
  /** Accent color in #RRGGBB format */
  accentHex: z.string().regex(/^#[0-9A-Fa-f]{6}$/, "Must be #RRGGBB format"),
  /** Texture id (from app/assets/textures) */
  textureId: z.string().min(1).optional(),
});

export type TabTheme = z.infer<typeof TabThemeSchema>;

/** Theme mode for global color temperature */
export const ThemeModeSchema = z.enum(["dark", "warm"]);

export type ThemeMode = z.infer<typeof ThemeModeSchema>;

/** Starred files for a single repo */
export const StarredFilesEntrySchema = z.object({
  docs: z.array(z.string()).default([]),
  code: z.array(z.string()).default([]),
});

export type StarredFilesEntry = z.infer<typeof StarredFilesEntrySchema>;

/** Starred files map: repoId -> { docs, code } */
export const StarredFilesSchema = z.record(z.string(), StarredFilesEntrySchema);

export type StarredFiles = z.infer<typeof StarredFilesSchema>;

/**
 * Full persisted configuration (saved to disk)
 */
export const PersistedConfigSchema = z.object({
  /** Schema version for migrations */
  configVersion: z.number().int().min(1).default(CONFIG_VERSION),
  /** Agent host */
  agentHost: z.string().min(1).default("127.0.0.1"),
  /** Agent port */
  agentPort: z.number().int().min(1024).max(65535).default(3141),
  /** Auto-start the WSL agent when the app launches */
  agentAutoStart: z.boolean().default(true),
  /** Optional WSL distro override for agent launch */
  agentDistro: z
    .preprocess((value) => {
      if (typeof value === "string") {
        const trimmed = value.trim();
        return trimmed.length > 0 ? trimmed : null;
      }
      return value;
    }, z.string().min(1).nullable())
    .default(null),
  /** Global auto-stage setting */
  autoStageGlobal: z.boolean().default(true),
  /** Configured repositories */
  repos: z.array(RepoConfigSchema).default([]),
  /** Maximum recent files to track per repo (25-2000) */
  recentFilesLimit: z.number().int().min(25).max(2000).default(200),
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
  /** Global classification excludes (used by Docs/Code panes only) */
  classificationExcludes: GlobalExcludesSchema.default({
    dirNames: [...GLOBAL_EXCLUDE_RECOMMENDED_DIRS],
    dirSuffixes: [...GLOBAL_EXCLUDE_RECOMMENDED_DIR_SUFFIXES],
    fileNames: [...GLOBAL_EXCLUDE_RECOMMENDED_FILES],
    extensions: [
      ...GLOBAL_EXCLUDE_RECOMMENDED_EXTENSIONS,
      ...GLOBAL_EXCLUDE_RECOMMENDED_FILE_SUFFIXES,
    ],
    patterns: [...GLOBAL_EXCLUDE_RECOMMENDED_PATTERNS],
  }),
  /** Custom output folder override (Windows path, null = default AppData) */
  outputWindowsRoot: z.string().nullable().default(null),
  /** Per-tab theme overrides, keyed by tabKey */
  tabThemes: z.record(z.string(), TabThemeSchema).default({}),
  /** Starred files per repo */
  starredFiles: StarredFilesSchema.default({}),
  /** Global theme mode (dark/warm) */
  themeMode: ThemeModeSchema.default("dark"),
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
  const normalizedRoots = normalizeLegacyRepoRoots(input);
  const normalizedCodeGlobs = normalizeLegacyCodeGlobs(normalizedRoots);
  const normalized = normalizeLegacyGlobalExcludes(normalizedCodeGlobs);
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
      lastActiveGroupRepoIds: {},
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
    classificationExcludes: {
      dirNames: [...GLOBAL_EXCLUDE_RECOMMENDED_DIRS],
      dirSuffixes: [...GLOBAL_EXCLUDE_RECOMMENDED_DIR_SUFFIXES],
      fileNames: [...GLOBAL_EXCLUDE_RECOMMENDED_FILES],
      extensions: [
        ...GLOBAL_EXCLUDE_RECOMMENDED_EXTENSIONS,
        ...GLOBAL_EXCLUDE_RECOMMENDED_FILE_SUFFIXES,
      ],
      patterns: [...GLOBAL_EXCLUDE_RECOMMENDED_PATTERNS],
    },
    outputWindowsRoot: null,
    agentAutoStart: true,
    agentDistro: null,
    tabThemes: {},
    starredFiles: {},
    themeMode: "dark",
  };
}
