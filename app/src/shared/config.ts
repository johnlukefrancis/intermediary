// Path: app/src/shared/config.ts
// Description: AppConfig Zod schema and types

import { z } from "zod";
import { TabIdSchema, WorktreeIdSchema } from "./protocol";

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
    },
    {
      repoId: "triangle-rain-tr-engine",
      label: "Triangle Rain (tr-engine)",
      wslPath: "/home/johnf/code/worktrees/tr-engine",
      tabId: "triangle-rain",
      worktreeId: "tr-engine",
      autoStage: true,
    },
    {
      repoId: "intermediary",
      label: "Intermediary",
      wslPath: "/home/johnf/code/intermediary",
      tabId: "intermediary",
      autoStage: true,
    },
  ],
});
