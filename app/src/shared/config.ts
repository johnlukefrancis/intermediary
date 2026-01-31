// Path: app/src/shared/config.ts
// Description: AppConfig Zod schema and types

import { z } from "zod";

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
  /** Display name in the UI */
  label: z.string(),
  /** Absolute WSL path to the repo root */
  wslPath: z.string(),
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
  repos: [
    {
      repoId: "textureportal",
      label: "TexturePortal",
      wslPath: "/home/johnf/code/textureportal",
      autoStage: true,
      docsGlobs: DEFAULT_DOCS_GLOBS,
      codeGlobs: DEFAULT_CODE_GLOBS,
      ignoreGlobs: DEFAULT_IGNORE_GLOBS,
      bundlePresets: [DEFAULT_BUNDLE_PRESET],
    },
    {
      repoId: "triangle-rain-tr-engine",
      label: "Triangle Rain",
      wslPath: "/home/johnf/code/worktrees/tr-engine",
      autoStage: true,
      docsGlobs: DEFAULT_DOCS_GLOBS,
      codeGlobs: DEFAULT_CODE_GLOBS,
      ignoreGlobs: DEFAULT_IGNORE_GLOBS,
      bundlePresets: [DEFAULT_BUNDLE_PRESET],
    },
    {
      repoId: "intermediary",
      label: "Intermediary",
      wslPath: "/home/johnf/code/intermediary",
      autoStage: true,
      docsGlobs: DEFAULT_DOCS_GLOBS,
      codeGlobs: DEFAULT_CODE_GLOBS,
      ignoreGlobs: DEFAULT_IGNORE_GLOBS,
      bundlePresets: [DEFAULT_BUNDLE_PRESET],
    },
  ],
});

// -----------------------------------------------------------------------------
// Persisted config (includes UI state and bundle selections)
// -----------------------------------------------------------------------------

/** Current config schema version */
export const CONFIG_VERSION = 3;

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
  const parsed = PersistedConfigSchema.parse(input);
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

  return { ...config, configVersion: CONFIG_VERSION };
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
