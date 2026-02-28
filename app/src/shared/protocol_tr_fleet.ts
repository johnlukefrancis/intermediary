// Path: app/src/shared/protocol_tr_fleet.ts
// Description: TR fleet command/response schemas for build-server status and recovery controls

import { z } from "zod";

export const TrFleetPortSchema = z.number().int().min(5601).max(5605);
export type TrFleetPort = z.infer<typeof TrFleetPortSchema>;

export const TrFleetWatchBackendSchema = z.enum(["auto", "native", "poll"]);
export type TrFleetWatchBackend = z.infer<typeof TrFleetWatchBackendSchema>;

export const GetTrFleetStatusCommandSchema = z.object({
  type: z.literal("getTrFleetStatus"),
});
export type GetTrFleetStatusCommand = z.infer<
  typeof GetTrFleetStatusCommandSchema
>;

export const TrFleetActionKindSchema = z.enum(["rebuild", "restartWatch"]);
export type TrFleetActionKind = z.infer<typeof TrFleetActionKindSchema>;

export const TrFleetActionCommandSchema = z.object({
  type: z.literal("trFleetAction"),
  action: TrFleetActionKindSchema,
  port: TrFleetPortSchema,
  backend: TrFleetWatchBackendSchema.optional(),
});
export type TrFleetActionCommand = z.infer<typeof TrFleetActionCommandSchema>;

export const TrFleetEndpointErrorCodeSchema = z.enum([
  "timeout",
  "unreachable",
  "http_error",
  "invalid_json",
  "unknown",
]);
export type TrFleetEndpointErrorCode = z.infer<
  typeof TrFleetEndpointErrorCodeSchema
>;

export const TrFleetEndpointErrorSchema = z.object({
  code: TrFleetEndpointErrorCodeSchema,
  message: z.string(),
  statusCode: z.number().int().min(100).max(599).optional(),
});
export type TrFleetEndpointError = z.infer<typeof TrFleetEndpointErrorSchema>;

export const TrFleetTargetStatusSchema = z.object({
  port: TrFleetPortSchema,
  baseUrl: z.string(),
  status: z.unknown().optional(),
  doctor: z.unknown().optional(),
  statusError: TrFleetEndpointErrorSchema.optional(),
  doctorError: TrFleetEndpointErrorSchema.optional(),
  fetchedAtMs: z.number().int().nonnegative(),
});
export type TrFleetTargetStatus = z.infer<typeof TrFleetTargetStatusSchema>;

export const GetTrFleetStatusResultSchema = z.object({
  type: z.literal("getTrFleetStatusResult"),
  targets: z.array(TrFleetTargetStatusSchema),
});
export type GetTrFleetStatusResult = z.infer<
  typeof GetTrFleetStatusResultSchema
>;

export const TrFleetActionResultSchema = z.object({
  type: z.literal("trFleetActionResult"),
  action: TrFleetActionKindSchema,
  port: TrFleetPortSchema,
  ok: z.boolean(),
  statusCode: z.number().int().min(100).max(599).optional(),
  responseBody: z.unknown().optional(),
  error: TrFleetEndpointErrorSchema.optional(),
});
export type TrFleetActionResult = z.infer<typeof TrFleetActionResultSchema>;
