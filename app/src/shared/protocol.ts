// Path: app/src/shared/protocol.ts
// Description: Agent<->UI WebSocket protocol types with Zod validation

import { z } from "zod";
import { AppConfigSchema } from "./config.js";
import { AgentEventSchema, type AgentEvent } from "./protocol_events.js";
import {
  BuildBundleCommandSchema,
  BuildBundleResultSchema,
  CancelBundleBuildCommandSchema,
  CancelBundleBuildResultSchema,
  ListBundlesResultSchema,
} from "./protocol_bundles.js";
import {
  GetTrFleetStatusCommandSchema,
  GetTrFleetStatusResultSchema,
  TrFleetActionCommandSchema,
  TrFleetActionResultSchema,
} from "./protocol_tr_fleet.js";
import {
  GetRepoTopLevelCommandSchema,
  GetRepoTopLevelResultSchema,
  ListRepoDirectoryCommandSchema,
  ListRepoDirectoryResultSchema,
} from "./protocol_repo_topology.js";
export {
  AgentErrorCodeSchema, AgentEventSchema, BundleBuildPhaseSchema, BundleBuildProgressEventSchema,
  BundleBuiltEventSchema, ErrorEventSchema, FileActivityBucketSchema, FileActivitySchema,
  FileChangeTypeSchema, FileChangedEventSchema,
  FileEntrySchema, FileKindSchema, HelloEventSchema, RepoTopologyChangedEventSchema,
  SnapshotEventSchema, StagedInfoSchema, WslBackendConnectionStatusSchema,
  WslBackendStatusEventSchema, type AgentErrorCode, type AgentErrorEvent, type AgentEvent,
  type BundleBuildPhase, type BundleBuildProgressEvent, type BundleBuiltEvent,
  type FileActivity, type FileActivityBucket, type FileChangeType, type FileChangedEvent,
  type FileEntry, type FileKind, type RepoTopologyChangedEvent, type StagedInfo,
  type WslBackendConnectionStatus,
  type WslBackendStatusEvent,
} from "./protocol_events.js";
export {
  BuildBundleCommandSchema, BuildBundleResultSchema, BundleInfoSchema, BundleSelectionSchema,
  CancelBundleBuildCommandSchema, CancelBundleBuildResultSchema, ListBundlesResultSchema,
  type BuildBundleResult, type BundleInfo, type BundleSelection, type CancelBundleBuildResult,
  type ListBundlesResult,
} from "./protocol_bundles.js";
export {
  GetTrFleetStatusCommandSchema, GetTrFleetStatusResultSchema, TrFleetActionCommandSchema,
  TrFleetActionResultSchema, TrFleetActionKindSchema, TrFleetEndpointErrorCodeSchema,
  TrFleetEndpointErrorSchema, TrFleetPortSchema, TrFleetTargetStatusSchema,
  TrFleetWatchBackendSchema, type GetTrFleetStatusCommand, type GetTrFleetStatusResult,
  type TrFleetActionCommand, type TrFleetActionKind, type TrFleetActionResult,
  type TrFleetEndpointError, type TrFleetEndpointErrorCode, type TrFleetPort,
  type TrFleetTargetStatus, type TrFleetWatchBackend,
} from "./protocol_tr_fleet.js";
export {
  GetRepoTopLevelCommandSchema, GetRepoTopLevelResultSchema,
  ListRepoDirectoryCommandSchema, ListRepoDirectoryResultSchema,
  type GetRepoTopLevelCommand, type GetRepoTopLevelResult,
  type ListRepoDirectoryCommand, type ListRepoDirectoryResult,
} from "./protocol_repo_topology.js";

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

export const UiCommandSchema = z.discriminatedUnion("type", [
  WatchRepoCommandSchema,
  RefreshCommandSchema,
  StageFileCommandSchema,
  ReadTextFileCommandSchema,
  ReadImageFileCommandSchema,
  BuildBundleCommandSchema,
  CancelBundleBuildCommandSchema,
  ClientHelloCommandSchema,
  SetOptionsCommandSchema,
  GetRepoTopLevelCommandSchema,
  ListRepoDirectoryCommandSchema,
  ListBundlesCommandSchema,
  GetTrFleetStatusCommandSchema,
  TrFleetActionCommandSchema,
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

export const UiResponseSchema = z.discriminatedUnion("type", [
  WatchRepoResultSchema,
  RefreshResultSchema,
  StageFileResultSchema,
  ReadTextFileResultSchema,
  ReadImageFileResultSchema,
  BuildBundleResultSchema,
  CancelBundleBuildResultSchema,
  ClientHelloResultSchema,
  SetOptionsResultSchema,
  GetRepoTopLevelResultSchema,
  ListRepoDirectoryResultSchema,
  ListBundlesResultSchema,
  GetTrFleetStatusResultSchema,
  TrFleetActionResultSchema,
]);
export type UiResponse = z.infer<typeof UiResponseSchema>;

// Individual result types for typed command helpers
export type WatchRepoResult = z.infer<typeof WatchRepoResultSchema>;
export type RefreshResult = z.infer<typeof RefreshResultSchema>;
export type StageFileResult = z.infer<typeof StageFileResultSchema>;
export type ReadTextFileResult = z.infer<typeof ReadTextFileResultSchema>;
export type ReadImageFileResult = z.infer<typeof ReadImageFileResultSchema>;
export type ClientHelloResult = z.infer<typeof ClientHelloResultSchema>;
export type SetOptionsResult = z.infer<typeof SetOptionsResultSchema>;

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
