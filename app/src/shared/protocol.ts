// Path: app/src/shared/protocol.ts
// Description: Agent<->UI WebSocket protocol types with Zod validation

import { z } from "zod";
import { AppConfigSchema } from "./config.js";

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
  wslPath: z.string(),
  windowsPath: z.string(),
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
  windowsPath: z.string(),
  aliasWindowsPath: z.string(),
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

export const ErrorEventSchema = z.object({
  type: z.literal("error"),
  scope: z.string(),
  message: z.string(),
  details: z.unknown().optional(),
});

export const AgentEventSchema = z.discriminatedUnion("type", [
  HelloEventSchema,
  FileChangedEventSchema,
  SnapshotEventSchema,
  BundleBuiltEventSchema,
  BundleBuildProgressEventSchema,
  ErrorEventSchema,
]);
export type AgentEvent = z.infer<typeof AgentEventSchema>;

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

/** Selection payload for bundle building */
export const BundleSelectionSchema = z.object({
  /** Whether to include root-level files */
  includeRoot: z.boolean(),
  /** Top-level directories to include */
  topLevelDirs: z.array(z.string().min(1)),
  /** Subdirectories to exclude (e.g. "TriangleRain/Assets") */
  excludedSubdirs: z.array(z.string().min(1)).default([]),
});
export type BundleSelection = z.infer<typeof BundleSelectionSchema>;

export const BuildBundleCommandSchema = z.object({
  type: z.literal("buildBundle"),
  repoId: z.string(),
  presetId: z.string(),
  selection: BundleSelectionSchema,
});

/** Handshake from UI with config and staging paths */
export const ClientHelloCommandSchema = z.object({
  type: z.literal("clientHello"),
  /** Full app configuration */
  config: AppConfigSchema,
  /** WSL path for staging files, e.g. /mnt/c/Users/.../staging/files */
  stagingWslRoot: z.string(),
  /** Windows path for staging files, e.g. C:\Users\...\staging\files */
  stagingWinRoot: z.string(),
  /** Whether to auto-stage docs/code files on change */
  autoStageOnChange: z.boolean().optional(),
});

/** Toggle agent options at runtime */
export const SetOptionsCommandSchema = z.object({
  type: z.literal("setOptions"),
  autoStageOnChange: z.boolean().optional(),
});

/** Request top-level directory listing for a repo */
export const GetRepoTopLevelCommandSchema = z.object({
  type: z.literal("getRepoTopLevel"),
  repoId: z.string(),
});

/** Request list of existing bundles for a preset */
export const ListBundlesCommandSchema = z.object({
  type: z.literal("listBundles"),
  repoId: z.string(),
  presetId: z.string(),
});

export const UiCommandSchema = z.discriminatedUnion("type", [
  WatchRepoCommandSchema,
  RefreshCommandSchema,
  StageFileCommandSchema,
  BuildBundleCommandSchema,
  ClientHelloCommandSchema,
  SetOptionsCommandSchema,
  GetRepoTopLevelCommandSchema,
  ListBundlesCommandSchema,
]);
export type UiCommand = z.infer<typeof UiCommandSchema>;

// -----------------------------------------------------------------------------
// UI -> Agent responses (payloads)
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
  windowsPath: z.string(),
  wslPath: z.string(),
  bytesCopied: z.number().int().nonnegative(),
  mtimeMs: z.number(),
});

export const BuildBundleResultSchema = z.object({
  type: z.literal("buildBundleResult"),
  repoId: z.string(),
  presetId: z.string(),
  windowsPath: z.string(),
  wslPath: z.string(),
  aliasWindowsPath: z.string(),
  bytes: z.number().int().nonnegative(),
  fileCount: z.number().int().nonnegative(),
  builtAtIso: z.string(),
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

/** Top-level directories and files for a repo */
export const GetRepoTopLevelResultSchema = z.object({
  type: z.literal("getRepoTopLevelResult"),
  repoId: z.string(),
  dirs: z.array(z.string()),
  files: z.array(z.string()),
  /** Subdirectories within each top-level dir (depth-2) */
  subdirs: z.record(z.string(), z.array(z.string())).optional(),
});

/** Info about a single bundle file */
export const BundleInfoSchema = z.object({
  windowsPath: z.string(),
  fileName: z.string(),
  bytes: z.number().int().nonnegative(),
  mtimeMs: z.number(),
  isLatestAlias: z.boolean(),
});
export type BundleInfo = z.infer<typeof BundleInfoSchema>;

/** Response with list of existing bundles */
export const ListBundlesResultSchema = z.object({
  type: z.literal("listBundlesResult"),
  repoId: z.string(),
  presetId: z.string(),
  bundles: z.array(BundleInfoSchema),
});

export const UiResponseSchema = z.discriminatedUnion("type", [
  WatchRepoResultSchema,
  RefreshResultSchema,
  StageFileResultSchema,
  BuildBundleResultSchema,
  ClientHelloResultSchema,
  SetOptionsResultSchema,
  GetRepoTopLevelResultSchema,
  ListBundlesResultSchema,
]);
export type UiResponse = z.infer<typeof UiResponseSchema>;

// Individual result types for typed command helpers
export type WatchRepoResult = z.infer<typeof WatchRepoResultSchema>;
export type RefreshResult = z.infer<typeof RefreshResultSchema>;
export type StageFileResult = z.infer<typeof StageFileResultSchema>;
export type BuildBundleResult = z.infer<typeof BuildBundleResultSchema>;
export type ClientHelloResult = z.infer<typeof ClientHelloResultSchema>;
export type SetOptionsResult = z.infer<typeof SetOptionsResultSchema>;
export type GetRepoTopLevelResult = z.infer<typeof GetRepoTopLevelResultSchema>;
export type ListBundlesResult = z.infer<typeof ListBundlesResultSchema>;

// -----------------------------------------------------------------------------
// Protocol envelopes
// -----------------------------------------------------------------------------

export const RequestEnvelopeSchema = z.object({
  kind: z.literal("request"),
  requestId: z.string(),
  payload: UiCommandSchema,
});
export type RequestEnvelope = z.infer<typeof RequestEnvelopeSchema>;

export const ResponseErrorSchema = z.object({
  code: z.string(),
  message: z.string(),
  details: z.unknown().optional(),
});
export type ResponseError = z.infer<typeof ResponseErrorSchema>;

export const ResponseOkEnvelopeSchema = z.object({
  kind: z.literal("response"),
  requestId: z.string(),
  status: z.literal("ok"),
  payload: UiResponseSchema,
});
export type ResponseOkEnvelope = z.infer<typeof ResponseOkEnvelopeSchema>;

export const ResponseErrorEnvelopeSchema = z.object({
  kind: z.literal("response"),
  requestId: z.string(),
  status: z.literal("error"),
  error: ResponseErrorSchema,
});
export type ResponseErrorEnvelope = z.infer<typeof ResponseErrorEnvelopeSchema>;

export const ResponseEnvelopeSchema = z.discriminatedUnion("status", [
  ResponseOkEnvelopeSchema,
  ResponseErrorEnvelopeSchema,
]);
export type ResponseEnvelope = z.infer<typeof ResponseEnvelopeSchema>;

export const EventEnvelopeSchema = z.object({
  kind: z.literal("event"),
  eventId: z.string().optional(),
  payload: AgentEventSchema,
});
export type EventEnvelope = z.infer<typeof EventEnvelopeSchema>;

export const ProtocolEnvelopeSchema = z.union([
  RequestEnvelopeSchema,
  ResponseEnvelopeSchema,
  EventEnvelopeSchema,
]);
export type ProtocolEnvelope = z.infer<typeof ProtocolEnvelopeSchema>;

// -----------------------------------------------------------------------------
// Parsing utilities
// -----------------------------------------------------------------------------

export function parseEnvelope(data: unknown): ProtocolEnvelope {
  return ProtocolEnvelopeSchema.parse(data);
}

export function parseAgentEvent(data: unknown): AgentEvent {
  return AgentEventSchema.parse(data);
}

export function parseUiCommand(data: unknown): UiCommand {
  return UiCommandSchema.parse(data);
}
