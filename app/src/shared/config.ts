// Path: app/src/shared/config.ts
// Description: AppConfig Zod schema and types

import { z } from "zod";

/**
 * Configuration for a single repository/worktree
 */
export const RepoConfigSchema = z.object({
  /** Unique identifier for this repo config */
  id: z.string(),
  /** Display name in the UI */
  name: z.string(),
  /** Absolute WSL path to the repo root */
  wslPath: z.string(),
  /** Which tab this repo belongs to */
  tabId: z.enum(["texture-portal", "triangle-rain", "intermediary"]),
  /** Optional worktree ID for Triangle Rain */
  worktreeId: z.string().optional(),
  /** Whether to auto-stage file changes */
  autoStage: z.boolean().default(true),
});

export type RepoConfig = z.infer<typeof RepoConfigSchema>;

/**
 * Global application configuration
 */
export const AppConfigSchema = z.object({
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
  return AppConfigSchema.parse({});
}
