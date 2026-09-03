// Path: app/src/shared/protocol_repo_commands.ts
// Description: Core repo watch, refresh, staging, file-read, handshake, and bundle-list command schemas

import { z } from "zod";
import { AppConfigSchema } from "./config.js";

// -----------------------------------------------------------------------------
// UI -> Agent commands (payloads)
// -----------------------------------------------------------------------------

export const WatchRepoCommandSchema = z.object({
  type: z.literal("watchRepo"),
  repoId: z.string(),
});

export const RefreshCommandSchema = z.object({
  type: z.literal("refresh"),
  repoId: z.string(),
});

export const StageFileCommandSchema = z.object({
  type: z.literal("stageFile"),
  repoId: z.string(),
  path: z.string(),
});

export const ReadTextFileCommandSchema = z.object({
  type: z.literal("readTextFile"),
  repoId: z.string(),
  path: z.string(),
});

export const ReadImageFileCommandSchema = z.object({
  type: z.literal("readImageFile"),
  repoId: z.string(),
  path: z.string(),
});

/** Handshake from UI with config and staging paths */
export const ClientHelloCommandSchema = z.object({
  type: z.literal("clientHello"),
  /** Full app configuration */
  config: AppConfigSchema,
  /** Host-native staging root path (Windows path on Windows, POSIX on macOS). */
  stagingHostRoot: z.string(),
  /** Legacy compatibility for agents that still expect stagingWinRoot. */
  stagingWinRoot: z.string().optional(),
  /** Optional WSL path for staging files (Windows + WSL bridge only). */
  stagingWslRoot: z.string().optional(),
  /** Whether to auto-stage classified feed files on change */
  autoStageOnChange: z.boolean().optional(),
});

/** Toggle agent options at runtime */
export const SetOptionsCommandSchema = z.object({
  type: z.literal("setOptions"),
  autoStageOnChange: z.boolean().optional(),
});

/** Request list of existing bundles for a preset */
export const ListBundlesCommandSchema = z.object({
  type: z.literal("listBundles"),
  repoId: z.string(),
  presetId: z.string(),
});

// -----------------------------------------------------------------------------
// Agent -> UI responses (payloads)
// -----------------------------------------------------------------------------

export const WatchRepoResultSchema = z.object({
  type: z.literal("watchRepoResult"),
  repoId: z.string(),
});

export const RefreshResultSchema = z.object({
  type: z.literal("refreshResult"),
  repoId: z.string(),
});

export const StageFileResultSchema = z.object({
  type: z.literal("stageFileResult"),
  repoId: z.string(),
  path: z.string(),
  hostPath: z.string(),
  wslPath: z.string().optional(),
  bytesCopied: z.number().int().nonnegative(),
  mtimeMs: z.number(),
});

export const ReadTextFileResultSchema = z.object({
  type: z.literal("readTextFileResult"),
  repoId: z.string(),
  path: z.string(),
  content: z.string(),
  bytes: z.number().int().nonnegative(),
  mtimeMs: z.number().int().nonnegative(),
  encoding: z.literal("utf-8"),
});

export const ReadImageFileResultSchema = z.object({
  type: z.literal("readImageFileResult"),
  repoId: z.string(),
  path: z.string(),
  dataBase64: z.string(),
  mimeType: z.string(),
  bytes: z.number().int().nonnegative(),
  mtimeMs: z.number().int().nonnegative(),
});

/** Response to clientHello with agent info */
export const ClientHelloResultSchema = z.object({
  type: z.literal("clientHelloResult"),
  agentVersion: z.string(),
  watchedRepoIds: z.array(z.string()),
});

/** Acknowledgment for setOptions */
export const SetOptionsResultSchema = z.object({
  type: z.literal("setOptionsResult"),
  autoStageOnChange: z.boolean(),
});

export type WatchRepoResult = z.infer<typeof WatchRepoResultSchema>;
export type RefreshResult = z.infer<typeof RefreshResultSchema>;
export type StageFileResult = z.infer<typeof StageFileResultSchema>;
export type ReadTextFileResult = z.infer<typeof ReadTextFileResultSchema>;
export type ReadImageFileResult = z.infer<typeof ReadImageFileResultSchema>;
export type ClientHelloResult = z.infer<typeof ClientHelloResultSchema>;
export type SetOptionsResult = z.infer<typeof SetOptionsResultSchema>;
