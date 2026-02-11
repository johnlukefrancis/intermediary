// Path: app/src/shared/protocol.ts
// Description: Agent<->UI WebSocket protocol types with Zod validation

import { z } from "zod";
import { AppConfigSchema } from "./config.js";
import { GlobalExcludesSchema } from "./global_excludes.js";
import { AgentEventSchema, type AgentEvent } from "./protocol_events.js";
export {
  AgentErrorCodeSchema,
  AgentEventSchema,
  BundleBuildPhaseSchema,
  BundleBuildProgressEventSchema,
  BundleBuiltEventSchema,
  ErrorEventSchema,
  FileChangeTypeSchema,
  FileChangedEventSchema,
  FileEntrySchema,
  FileKindSchema,
  HelloEventSchema,
  SnapshotEventSchema,
  StagedInfoSchema,
  WslBackendConnectionStatusSchema,
  WslBackendStatusEventSchema,
  type AgentErrorCode,
  type AgentErrorEvent,
  type AgentEvent,
  type BundleBuildPhase,
  type BundleBuildProgressEvent,
  type BundleBuiltEvent,
  type FileChangeType,
  type FileChangedEvent,
  type FileEntry,
  type FileKind,
  type StagedInfo,
  type WslBackendConnectionStatus,
  type WslBackendStatusEvent,
} from "./protocol_events.js";

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
  /** Global excludes (extensions and patterns) */
  globalExcludes: GlobalExcludesSchema.optional(),
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
  hostPath: z.string(),
  wslPath: z.string().optional(),
  bytesCopied: z.number().int().nonnegative(),
  mtimeMs: z.number(),
});

export const BuildBundleResultSchema = z.object({
  type: z.literal("buildBundleResult"),
  repoId: z.string(),
  presetId: z.string(),
  hostPath: z.string(),
  wslPath: z.string().optional(),
  aliasHostPath: z.string(),
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
  /** Dir names that are excluded by default (e.g. node_modules, .git, target) */
  defaultExcluded: z.array(z.string()).default([]),
});

/** Info about a single bundle file */
export const BundleInfoSchema = z.object({
  hostPath: z.string(),
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
