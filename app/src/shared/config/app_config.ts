// Path: app/src/shared/config/app_config.ts
// Description: AppConfig schema, types, and defaults

import { z } from "zod";
import { RepoConfigSchema, type RepoConfig } from "./repo_config.js";
import {
  GlobalExcludesSchema,
  GLOBAL_EXCLUDE_RECOMMENDED_DIRS,
  GLOBAL_EXCLUDE_RECOMMENDED_DIR_SUFFIXES,
  GLOBAL_EXCLUDE_RECOMMENDED_EXTENSIONS,
  GLOBAL_EXCLUDE_RECOMMENDED_FILE_SUFFIXES,
  GLOBAL_EXCLUDE_RECOMMENDED_FILES,
  GLOBAL_EXCLUDE_RECOMMENDED_PATTERNS,
  type GlobalExcludes,
} from "../global_excludes.js";

/**
 * Global application configuration
 */
export const AppConfigSchema = z.object({
  /** Hostname for the agent WebSocket server */
  agentHost: z.string().min(1).default("127.0.0.1"),
  /** Port the agent WebSocket server listens on */
  agentPort: z.number().int().min(1024).max(65535).default(3141),
  /** Global default for auto-staging */
  autoStageGlobal: z.boolean().default(true),
  /** Configured repositories */
  repos: z.array(RepoConfigSchema).default([]),
  /** Maximum recent files to track per repo (25-2000) */
  recentFilesLimit: z.number().int().min(25).max(2000).default(40),
  /** Global classification excludes for Docs/Code pane filtering */
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
});

export type AppConfig = z.infer<typeof AppConfigSchema>;

export const DEFAULT_APP_CONFIG: AppConfig = AppConfigSchema.parse({
  agentHost: "127.0.0.1",
  agentPort: 3141,
  autoStageGlobal: true,
  repos: [], // Empty by default - users add repos via the UI
  recentFilesLimit: 40,
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
});

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

export function extractAppConfig(persisted: {
  agentHost: string;
  agentPort: number;
  autoStageGlobal: boolean;
  repos: RepoConfig[];
  recentFilesLimit: number;
  classificationExcludes: GlobalExcludes;
}): AppConfig {
  return {
    agentHost: persisted.agentHost,
    agentPort: persisted.agentPort,
    autoStageGlobal: persisted.autoStageGlobal,
    repos: persisted.repos,
    recentFilesLimit: persisted.recentFilesLimit,
    classificationExcludes: persisted.classificationExcludes,
  };
}
