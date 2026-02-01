// Path: app/src/shared/config/bundle_presets.ts
// Description: Bundle preset schema, type, and defaults

import { z } from "zod";

/**
 * Configuration for a bundle preset
 */
export const BundlePresetSchema = z.object({
  /** Unique identifier for this preset */
  presetId: z.string().min(1),
  /** Display name in the UI */
  presetName: z.string().min(1),
  /** Whether to include root-level files */
  includeRoot: z.boolean().default(true),
  /** Top-level directories to include (empty = default to all at runtime) */
  topLevelDirs: z.array(z.string().min(1)).default([]),
});

export type BundlePreset = z.infer<typeof BundlePresetSchema>;

export const DEFAULT_BUNDLE_PRESET: BundlePreset = {
  presetId: "context",
  presetName: "Context",
  includeRoot: true,
  topLevelDirs: [],
};
