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

export const FileKindSchema = z.enum(["docs", "code", "other"]);
export type FileKind = z.infer<typeof FileKindSchema>;

export const FileEntrySchema = z.object({
  /** Relative path from repo root */
  path: z.string(),
  /** Classification for column routing */
  kind: FileKindSchema,
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

export const FileChangedEventSchema = z.object({
  type: z.literal("fileChanged"),
  repoId: z.string(),
  path: z.string(),
  kind: FileKindSchema,
  mtime: z.string(),
});

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
  sizeBytes: z.number().int().nonnegative(),
  mtime: z.string(),
  gitShort: z.string(),
});

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

export const BuildBundleCommandSchema = z.object({
  type: z.literal("buildBundle"),
  repoId: z.string(),
  presetId: z.string(),
});

export const UiCommandSchema = z.discriminatedUnion("type", [
  WatchRepoCommandSchema,
  RefreshCommandSchema,
  StageFileCommandSchema,
  BuildBundleCommandSchema,
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
});

export const BuildBundleResultSchema = z.object({
  type: z.literal("buildBundleResult"),
  repoId: z.string(),
  presetId: z.string(),
  windowsPath: z.string(),
});

export const UiResponseSchema = z.discriminatedUnion("type", [
  WatchRepoResultSchema,
  RefreshResultSchema,
  StageFileResultSchema,
  BuildBundleResultSchema,
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

export const ProtocolEnvelopeSchema = z.discriminatedUnion("kind", [
  RequestEnvelopeSchema,
  ResponseOkEnvelopeSchema,
  ResponseErrorEnvelopeSchema,
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
