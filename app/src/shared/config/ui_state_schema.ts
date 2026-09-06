// Path: app/src/shared/config/ui_state_schema.ts
// Description: Persisted UI state schema: rail section, left panel mode, window bounds

import { z } from "zod";

/** Window bounds persisted for a specific UI mode */
export const UiWindowBoundsSchema = z.object({
  width: z.number().int().min(1),
  height: z.number().int().min(1),
});

export type UiWindowBounds = z.infer<typeof UiWindowBoundsSchema>;

/** Optional per-mode window bounds */
export const UiWindowBoundsByModeSchema = z.object({
  standard: UiWindowBoundsSchema.optional(),
  handset: UiWindowBoundsSchema.optional(),
});

export type UiWindowBoundsByMode = z.infer<typeof UiWindowBoundsByModeSchema>;

/** Right-rail instrument selection: zip bundles, source control, or the terminal.
 *  An unknown value (a config written by a newer build) falls back to ZIPS instead of locking
 *  persistence for the session, like `UiModeSchema`. */
export const ActiveRailSchema = z.enum(["zips", "source", "terminal"]).catch("zips");

export type ActiveRail = z.infer<typeof ActiveRailSchema>;

/** Left file panel modes in rocker order; STREAM is first and is the default */
export const FILES_MODES = ["stream", "auto", "latest", "active"] as const;

/** Left file panel mode: the live Stream or one of the three table sorts.
 *  Unknown values fall back to STREAM, so a config from a newer build never locks persistence. */
export const FilesModeSchema = z.enum(FILES_MODES).catch("stream");

export type FilesMode = z.infer<typeof FilesModeSchema>;

/** Remembered UI state */
export const UiStateSchema = z.object({
  /** Last active repo (by repoId) */
  lastActiveTabId: z.string().nullable().default(null),
  /** Last active repo per group (groupId -> repoId) */
  lastActiveGroupRepoIds: z.record(z.string(), z.string()).default({}),
  /** Remembered window bounds by UI mode */
  windowBoundsByMode: UiWindowBoundsByModeSchema.default({}),
  /** Right-rail section shown in the deck (defaulted, so no migration is needed) */
  activeRail: ActiveRailSchema.default("zips"),
  /** Left file panel mode (defaulted, so no migration is needed) */
  filesMode: FilesModeSchema.default("stream"),
  /** Rail share of the deck width in standard layout, set by the drag divider (20-70) */
  railWidthPercent: z.number().int().min(20).max(70).catch(35),
});

export type UiState = z.infer<typeof UiStateSchema>;
