// Path: app/src/shared/protocol.ts
// Description: Agent<->UI WebSocket protocol types with Zod validation

import { z } from "zod";

// -----------------------------------------------------------------------------
// Tab and Worktree identifiers
// -----------------------------------------------------------------------------

export const TabIdSchema = z.enum([
  "texture-portal",
  "triangle-rain",
  "intermediary",
]);
export type TabId = z.infer<typeof TabIdSchema>;

export const WorktreeIdSchema = z.enum(["tr-engine"]);
export type WorktreeId = z.infer<typeof WorktreeIdSchema>;

// -----------------------------------------------------------------------------
// File metadata
// -----------------------------------------------------------------------------

export const FileMetaSchema = z.object({
  /** Relative path from repo root */
  path: z.string(),
  /** File size in bytes */
  size: z.number().int().nonnegative(),
  /** Last modified timestamp (ISO 8601) */
  mtime: z.string(),
  /** Whether the file is staged for bundling */
  staged: z.boolean(),
});
export type FileMeta = z.infer<typeof FileMetaSchema>;

// -----------------------------------------------------------------------------
// Agent -> UI messages
// -----------------------------------------------------------------------------

export const HelloMessageSchema = z.object({
  type: z.literal("hello"),
  agentVersion: z.string(),
  timestamp: z.string(),
});

export const FileChangedMessageSchema = z.object({
  type: z.literal("fileChanged"),
  repoId: z.string(),
  file: FileMetaSchema,
  changeType: z.enum(["created", "modified", "deleted"]),
});

export const SnapshotMessageSchema = z.object({
  type: z.literal("snapshot"),
  repoId: z.string(),
  files: z.array(FileMetaSchema),
});

export const BundleBuiltMessageSchema = z.object({
  type: z.literal("bundleBuilt"),
  repoId: z.string(),
  bundlePath: z.string(),
  fileCount: z.number().int().nonnegative(),
  sizeBytes: z.number().int().nonnegative(),
});

export const ErrorMessageSchema = z.object({
  type: z.literal("error"),
  code: z.string(),
  message: z.string(),
  details: z.unknown().optional(),
});

export const AgentMessageSchema = z.discriminatedUnion("type", [
  HelloMessageSchema,
  FileChangedMessageSchema,
  SnapshotMessageSchema,
  BundleBuiltMessageSchema,
  ErrorMessageSchema,
]);
export type AgentMessage = z.infer<typeof AgentMessageSchema>;

// -----------------------------------------------------------------------------
// UI -> Agent messages
// -----------------------------------------------------------------------------

export const WatchRepoCommandSchema = z.object({
  type: z.literal("watchRepo"),
  repoId: z.string(),
  wslPath: z.string(),
});

export const RefreshCommandSchema = z.object({
  type: z.literal("refresh"),
  repoId: z.string(),
});

export const StageFileCommandSchema = z.object({
  type: z.literal("stageFile"),
  repoId: z.string(),
  path: z.string(),
  staged: z.boolean(),
});

export const BuildBundleCommandSchema = z.object({
  type: z.literal("buildBundle"),
  repoId: z.string(),
  outputName: z.string(),
});

export const UiCommandSchema = z.discriminatedUnion("type", [
  WatchRepoCommandSchema,
  RefreshCommandSchema,
  StageFileCommandSchema,
  BuildBundleCommandSchema,
]);
export type UiCommand = z.infer<typeof UiCommandSchema>;

// -----------------------------------------------------------------------------
// Parsing utilities
// -----------------------------------------------------------------------------

export function parseAgentMessage(data: unknown): AgentMessage {
  return AgentMessageSchema.parse(data);
}

export function parseUiCommand(data: unknown): UiCommand {
  return UiCommandSchema.parse(data);
}
