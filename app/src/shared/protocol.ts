// Path: app/src/shared/protocol.ts
// Description: Agent<->UI WebSocket protocol unions and envelopes with Zod validation

import { z } from "zod";
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
import {
  ClientHelloCommandSchema, ClientHelloResultSchema, ListBundlesCommandSchema,
  ReadImageFileCommandSchema, ReadImageFileResultSchema, ReadTextFileCommandSchema,
  ReadTextFileResultSchema, RefreshCommandSchema, RefreshResultSchema, SetOptionsCommandSchema,
  SetOptionsResultSchema, StageFileCommandSchema, StageFileResultSchema, WatchRepoCommandSchema,
  WatchRepoResultSchema,
} from "./protocol_repo_commands.js";
import {
  SourceControlActionCommandSchema, SourceControlActionResultSchema,
  SourceControlDiffCommandSchema, SourceControlDiffResultSchema,
  SourceControlImageDiffCommandSchema, SourceControlImageDiffResultSchema,
  SourceControlStatusCommandSchema, SourceControlStatusResultSchema,
} from "./protocol_source_control.js";
export {
  AgentErrorCodeSchema, AgentEventSchema, BundleBuildPhaseSchema, BundleBuildProgressEventSchema,
  BundleBuiltEventSchema, ErrorEventSchema, FileActivityBucketSchema, FileActivitySchema,
  FileChangeTypeSchema, FileChangedEventSchema,
  FileEntrySchema, FileKindSchema, HelloEventSchema, RepoTopologyChangedEventSchema,
  SnapshotEventSchema, SourceControlChangedEventSchema, StagedInfoSchema,
  WslBackendConnectionStatusSchema, WslBackendStatusEventSchema,
  type AgentErrorCode, type AgentErrorEvent, type AgentEvent,
  type BundleBuildPhase, type BundleBuildProgressEvent, type BundleBuiltEvent,
  type FileActivity, type FileActivityBucket, type FileChangeType, type FileChangedEvent,
  type FileEntry, type FileKind, type RepoTopologyChangedEvent, type SourceControlChangedEvent,
  type StagedInfo, type WslBackendConnectionStatus, type WslBackendStatusEvent,
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
export {
  ClientHelloCommandSchema, ClientHelloResultSchema, ListBundlesCommandSchema,
  ReadImageFileCommandSchema, ReadImageFileResultSchema, ReadTextFileCommandSchema,
  ReadTextFileResultSchema, RefreshCommandSchema, RefreshResultSchema, SetOptionsCommandSchema,
  SetOptionsResultSchema, StageFileCommandSchema, StageFileResultSchema, WatchRepoCommandSchema,
  WatchRepoResultSchema, type ClientHelloResult, type ReadImageFileResult, type ReadTextFileResult,
  type RefreshResult, type SetOptionsResult, type StageFileResult, type WatchRepoResult,
} from "./protocol_repo_commands.js";
export {
  AGENT_DRAINING_CODE, ImageDiffSideSchema, ImageDiffSourceSchema,
  SOURCE_CONTROL_STATE_CHANGED_CODE, SOURCE_CONTROL_UNSUPPORTED_LAYOUT_CODE,
  SourceControlActionCommandSchema, SourceControlActionKindSchema, SourceControlActionResultSchema,
  SourceControlActionSchema, SourceControlAreaSchema, SourceControlChangeSchema,
  SourceControlDiffCommandSchema, SourceControlDiffResultSchema, SourceControlDiscardTargetSchema,
  SourceControlEffectSchema, SourceControlEntryAreaSchema, SourceControlEntrySchema,
  SourceControlErrorDetailsSchema, SourceControlImageDiffCommandSchema,
  SourceControlImageDiffResultSchema, SourceControlOmittedSchema, SourceControlScopeSchema,
  SourceControlStampSchema, SourceControlStatusCommandSchema, SourceControlStatusResultSchema,
  SourceControlStatusSchema,
  type ImageDiffSide, type ImageDiffSource,
  type SourceControlAction, type SourceControlActionCommand, type SourceControlActionKind,
  type SourceControlActionResult, type SourceControlArea, type SourceControlChange,
  type SourceControlDiffCommand, type SourceControlDiffResult, type SourceControlDiscardTarget,
  type SourceControlEffect, type SourceControlEntry, type SourceControlEntryArea,
  type SourceControlErrorDetails, type SourceControlImageDiffCommand,
  type SourceControlImageDiffResult, type SourceControlOmitted, type SourceControlScope,
  type SourceControlStamp, type SourceControlStatus, type SourceControlStatusCommand,
  type SourceControlStatusResult,
} from "./protocol_source_control.js";

// -----------------------------------------------------------------------------
// Unions
// -----------------------------------------------------------------------------

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
  SourceControlStatusCommandSchema,
  SourceControlDiffCommandSchema,
  SourceControlImageDiffCommandSchema,
  SourceControlActionCommandSchema,
]);
export type UiCommand = z.infer<typeof UiCommandSchema>;

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
  SourceControlStatusResultSchema,
  SourceControlDiffResultSchema,
  SourceControlImageDiffResultSchema,
  SourceControlActionResultSchema,
]);
export type UiResponse = z.infer<typeof UiResponseSchema>;

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
