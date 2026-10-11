// Path: app/src/shared/protocol_bundle_inspection.ts
// Description: Selection-aware bundle entry measurements and bounded largest-file results

import { z } from "zod";
import { BundleSelectionSchema } from "./protocol_bundles.js";
import { GlobalExcludesSchema } from "./global_excludes.js";

export const InspectBundleEntryCommandSchema = z.object({
  type: z.literal("inspectBundleEntry"),
  repoId: z.string(),
  path: z.string().min(1),
  selection: BundleSelectionSchema,
  globalExcludes: GlobalExcludesSchema,
});

export const InspectBundleEntryResultSchema = z.object({
  type: z.literal("inspectBundleEntryResult"),
  repoId: z.string(),
  path: z.string(),
  inspection: z.object({
    isDirectory: z.boolean(),
    bytes: z.number().nonnegative(),
    fileCount: z.number().int().nonnegative(),
    includedBytes: z.number().nonnegative(),
    includedFileCount: z.number().int().nonnegative(),
    largestIncludedFiles: z.array(z.object({ path: z.string(), bytes: z.number().nonnegative() })).max(5),
    modifiedMs: z.number().nullable(),
    complete: z.boolean(),
    unreadableEntries: z.number().int().nonnegative(),
  }),
});

export type BundleEntryInspection = z.infer<typeof InspectBundleEntryResultSchema>["inspection"];
export type InspectBundleEntryResult = z.infer<typeof InspectBundleEntryResultSchema>;
