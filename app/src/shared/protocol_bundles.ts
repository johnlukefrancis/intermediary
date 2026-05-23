// Path: app/src/shared/protocol_bundles.ts
// Description: Bundle-related agent protocol schemas and types

import { z } from "zod";
import { GlobalExcludesSchema } from "./global_excludes.js";

export const BundleSelectionSchema = z.object({
  /** Whether to include root-level files */
  includeRoot: z.boolean(),
  /** Top-level directories to include */
  topLevelDirs: z.array(z.string().min(1)),
  /** Repo-relative subdirectory paths to exclude (e.g. "TriangleRain/Assets") */
  excludedSubdirs: z.array(z.string().min(1)).default([]),
  /** Repo-relative file paths to exclude inside the selected roots/directories */
  excludedFiles: z.array(z.string().min(1)).default([]),
});
export type BundleSelection = z.infer<typeof BundleSelectionSchema>;

export const BuildBundleCommandSchema = z.object({
  type: z.literal("buildBundle"),
  repoId: z.string(),
  presetId: z.string(),
  buildId: z.string(),
  selection: BundleSelectionSchema,
  /** Global excludes (extensions and patterns) */
  globalExcludes: GlobalExcludesSchema.optional(),
});

export const CancelBundleBuildCommandSchema = z.object({
  type: z.literal("cancelBundleBuild"),
  repoId: z.string(),
  presetId: z.string(),
  buildId: z.string(),
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
export type BuildBundleResult = z.infer<typeof BuildBundleResultSchema>;

export const CancelBundleBuildResultSchema = z.object({
  type: z.literal("cancelBundleBuildResult"),
  repoId: z.string(),
  presetId: z.string(),
  buildId: z.string(),
  cancelled: z.boolean(),
});
export type CancelBundleBuildResult = z.infer<typeof CancelBundleBuildResultSchema>;

export const BundleInfoSchema = z.object({
  hostPath: z.string(),
  fileName: z.string(),
  bytes: z.number().int().nonnegative(),
  mtimeMs: z.number(),
  isLatestAlias: z.boolean(),
});
export type BundleInfo = z.infer<typeof BundleInfoSchema>;

export const ListBundlesResultSchema = z.object({
  type: z.literal("listBundlesResult"),
  repoId: z.string(),
  presetId: z.string(),
  bundles: z.array(BundleInfoSchema),
});
export type ListBundlesResult = z.infer<typeof ListBundlesResultSchema>;
