// Path: app/src/shared/protocol_events_delta.ts
// Description: Zod mirror of the agent fileDelta event: bounded content of one settled file change

import { z } from "zod";
import { FileKindSchema } from "./protocol_file_meta.js";

/** What the agent observed happening to the path since its previous sighting */
export const DeltaOpSchema = z.enum(["add", "modify", "remove", "rename"]);
export type DeltaOp = z.infer<typeof DeltaOpSchema>;

/**
 * What the patch is measured against. `previousSighting` = the text this agent process last
 * served for the path; `index` = the staged blob (first sighting of a tracked path);
 * `none` = no baseline, so the patch is all-added.
 */
export const DeltaBaselineSchema = z.enum(["previousSighting", "index", "none"]);
export type DeltaBaseline = z.infer<typeof DeltaBaselineSchema>;

export const DeltaStatsSchema = z.object({
  added: z.number().int().nonnegative(),
  removed: z.number().int().nonnegative(),
  hunks: z.number().int().nonnegative(),
  newLines: z.number().int().nonnegative(),
});
export type DeltaStats = z.infer<typeof DeltaStatsSchema>;

/** Why the agent shipped no text for a path it classified as text */
export const OpaqueReasonSchema = z.enum(["binary", "tooLarge", "unreadable"]);
export type OpaqueReason = z.infer<typeof OpaqueReasonSchema>;

export const DeltaPayloadSchema = z.discriminatedUnion("kind", [
  z.object({
    kind: z.literal("text"),
    patch: z.string(),
    stats: DeltaStatsSchema,
    baseline: DeltaBaselineSchema,
    truncated: z.boolean(),
  }),
  // Pixels are fetched by the UI via readImageFile under its own size gate; none cross here
  z.object({
    kind: z.literal("image"),
    bytes: z.number().int().nonnegative(),
    mimeType: z.string().nullable(),
    /** The file's mtime (ms) at the metadata read that produced the event: the revision the tile shows */
    mtimeMs: z.number().int().nonnegative(),
  }),
  z.object({
    kind: z.literal("opaque"),
    bytes: z.number().int().nonnegative(),
    reason: OpaqueReasonSchema,
  }),
  z.object({ kind: z.literal("gone") }),
]);
export type DeltaPayload = z.infer<typeof DeltaPayloadSchema>;

export const FileDeltaEventSchema = z.object({
  type: z.literal("fileDelta"),
  repoId: z.string(),
  /** Strictly increasing per repo per agent process; a gap means the event bus dropped */
  seq: z.number().int().nonnegative(),
  path: z.string(),
  fromPath: z.string().optional(),
  kind: FileKindSchema,
  op: DeltaOpSchema,
  mtime: z.string(),
  /** Best-effort: the tracked set reloads up to ~1 s behind the index */
  tracked: z.boolean().optional(),
  /** Counters accumulated since the previous emitted delta for this repo */
  folded: z.number().int().nonnegative(),
  withheld: z.number().int().nonnegative(),
  dropped: z.number().int().nonnegative(),
  payload: DeltaPayloadSchema,
});
export type FileDeltaEvent = z.infer<typeof FileDeltaEventSchema>;

/**
 * Counters the agent would otherwise strand: withheld and dropped accumulated since the previous
 * emitted delta, published when its queue goes quiet or a burst window closes without a delta.
 */
export const FileDeltaCountersEventSchema = z.object({
  type: z.literal("fileDeltaCounters"),
  repoId: z.string(),
  /** Consumed from the same per-repo sequence as fileDelta, so a drop before it still shows as a gap */
  seq: z.number().int().nonnegative(),
  withheld: z.number().int().nonnegative(),
  dropped: z.number().int().nonnegative(),
});
export type FileDeltaCountersEvent = z.infer<typeof FileDeltaCountersEventSchema>;
