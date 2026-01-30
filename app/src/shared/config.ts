// Path: app/src/shared/config.ts
// Description: AppConfig Zod schema and types

import { z } from "zod";
import { TabIdSchema, WorktreeIdSchema } from "./ids.js";

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
  /** Top-level directories to include (empty = ALL) */
  topLevelDirs: z.array(z.string()).default([]),
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
];

/**
 * Configuration for a single repository/worktree
 */
export const RepoConfigSchema = z.object({
  /** Unique identifier for this repo config */
  repoId: z.string(),
  /** Display name in the UI */
  label: z.string(),
  /** Absolute WSL path to the repo root */
  wslPath: z.string(),
  /** Which tab this repo belongs to */
  tabId: TabIdSchema,
  /** Optional worktree ID for Triangle Rain */
  worktreeId: WorktreeIdSchema.optional(),
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
      tabId: "texture-portal",
      autoStage: true,
      docsGlobs: DEFAULT_DOCS_GLOBS,
      codeGlobs: DEFAULT_CODE_GLOBS,
      ignoreGlobs: DEFAULT_IGNORE_GLOBS,
      bundlePresets: [DEFAULT_BUNDLE_PRESET],
    },
    {
      repoId: "triangle-rain-tr-engine",
      label: "Triangle Rain (tr-engine)",
      wslPath: "/home/johnf/code/worktrees/tr-engine",
      tabId: "triangle-rain",
      worktreeId: "tr-engine",
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
      tabId: "intermediary",
      autoStage: true,
      docsGlobs: DEFAULT_DOCS_GLOBS,
      codeGlobs: DEFAULT_CODE_GLOBS,
      ignoreGlobs: DEFAULT_IGNORE_GLOBS,
      bundlePresets: [DEFAULT_BUNDLE_PRESET],
    },
  ],
});
