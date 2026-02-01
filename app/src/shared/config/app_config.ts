// Path: app/src/shared/config/app_config.ts
// Description: AppConfig schema, types, and defaults

import { z } from "zod";
import { RepoConfigSchema, type RepoConfig } from "./repo_config.js";

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

export const DEFAULT_APP_CONFIG: AppConfig = AppConfigSchema.parse({
  agentHost: "localhost",
  agentPort: 3141,
  autoStageGlobal: true,
  repos: [], // Empty by default - users add repos via the UI
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
}): AppConfig {
  return {
    agentHost: persisted.agentHost,
    agentPort: persisted.agentPort,
    autoStageGlobal: persisted.autoStageGlobal,
    repos: persisted.repos,
  };
}
