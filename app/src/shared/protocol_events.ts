// Path: app/src/shared/protocol_events.ts
// Description: Agent event and file metadata schemas shared by protocol envelope parsing

import { z } from "zod";

// -----------------------------------------------------------------------------
// File metadata
// -----------------------------------------------------------------------------

export const FileKindSchema = z.enum(["docs", "code", "other"]);
export type FileKind = z.infer<typeof FileKindSchema>;

export const FileChangeTypeSchema = z.enum(["add", "change", "unlink"]);
export type FileChangeType = z.infer<typeof FileChangeTypeSchema>;

export const FileEntrySchema = z.object({
  /** Relative path from repo root */
  path: z.string(),
  /** Classification for column routing */
  kind: FileKindSchema,
  /** Last observed change type for this file */
  changeType: FileChangeTypeSchema,
  /** Last modified timestamp (ISO 8601) */
  mtime: z.string(),
  /** Optional file size in bytes */
  sizeBytes: z.number().int().nonnegative().optional(),
});
export type FileEntry = z.infer<typeof FileEntrySchema>;

// -----------------------------------------------------------------------------
// Agent -> UI events (payloads)
// -----------------------------------------------------------------------------

export const HelloEventSchema = z.object({
  type: z.literal("hello"),
  agentVersion: z.string(),
  distro: z.string(),
  reposDetected: z.array(z.string()).optional(),
});

/** Staging info attached to file change events when auto-staged */
export const StagedInfoSchema = z.object({
  hostPath: z.string(),
  wslPath: z.string().optional(),
  bytesCopied: z.number().int().nonnegative(),
  mtimeMs: z.number(),
});
export type StagedInfo = z.infer<typeof StagedInfoSchema>;

export const FileChangedEventSchema = z.object({
  type: z.literal("fileChanged"),
  repoId: z.string(),
  path: z.string(),
  kind: FileKindSchema,
  changeType: FileChangeTypeSchema,
  mtime: z.string(),
  /** Present when file was auto-staged */
  staged: StagedInfoSchema.optional(),
});
export type FileChangedEvent = z.infer<typeof FileChangedEventSchema>;

export const SnapshotEventSchema = z.object({
  type: z.literal("snapshot"),
  repoId: z.string(),
  recent: z.array(FileEntrySchema),
});

export const BundleBuiltEventSchema = z.object({
  type: z.literal("bundleBuilt"),
  repoId: z.string(),
  presetId: z.string(),
  hostPath: z.string(),
  aliasHostPath: z.string(),
  bytes: z.number().int().nonnegative(),
  fileCount: z.number().int().nonnegative(),
  builtAtIso: z.string(),
});
export type BundleBuiltEvent = z.infer<typeof BundleBuiltEventSchema>;

export const BundleBuildPhaseSchema = z.enum(["scanning", "zipping", "finalizing"]);
export type BundleBuildPhase = z.infer<typeof BundleBuildPhaseSchema>;

export const BundleBuildProgressEventSchema = z.object({
  type: z.literal("bundleBuildProgress"),
  repoId: z.string(),
  presetId: z.string(),
  phase: BundleBuildPhaseSchema,
  filesDone: z.number().int().nonnegative(),
  filesTotal: z.number().int().nonnegative(),
  currentFile: z.string().optional(),
  currentBytesDone: z.number().int().nonnegative().optional(),
  currentBytesTotal: z.number().int().nonnegative().optional(),
  bytesDoneTotalBestEffort: z.number().int().nonnegative().optional(),
});
export type BundleBuildProgressEvent = z.infer<typeof BundleBuildProgressEventSchema>;

export const AgentErrorCodeSchema = z.enum(["watcher_inotify_limit", "watcher_fd_limit"]);
export type AgentErrorCode = z.infer<typeof AgentErrorCodeSchema>;

export const ErrorEventSchema = z.object({
  type: z.literal("error"),
  scope: z.string(),
  message: z.string(),
  details: z
    .object({
      code: AgentErrorCodeSchema.optional(),
      docPath: z.string().optional(),
      repoId: z.string().optional(),
      rawCode: z.string().optional(),
      rawMessage: z.string().optional(),
    })
    .optional(),
});
export type AgentErrorEvent = z.infer<typeof ErrorEventSchema>;

export const WslBackendConnectionStatusSchema = z.enum(["online", "offline"]);
export type WslBackendConnectionStatus = z.infer<
  typeof WslBackendConnectionStatusSchema
>;

export const WslBackendStatusEventSchema = z.object({
  type: z.literal("wslBackendStatus"),
  status: WslBackendConnectionStatusSchema,
  generation: z.number().int().nonnegative(),
});
export type WslBackendStatusEvent = z.infer<typeof WslBackendStatusEventSchema>;

export const AgentEventSchema = z.discriminatedUnion("type", [
  HelloEventSchema,
  FileChangedEventSchema,
  SnapshotEventSchema,
  BundleBuiltEventSchema,
  BundleBuildProgressEventSchema,
  ErrorEventSchema,
  WslBackendStatusEventSchema,
]);
export type AgentEvent = z.infer<typeof AgentEventSchema>;
