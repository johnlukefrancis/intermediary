// Path: app/src/lib/files/files_mode.ts
// Description: The left file panel's mode vocabulary: the live Stream plus the three table sort modes

import type { FileSortMode } from "./file_feed.js";
import type { FilesMode } from "../../shared/config/ui_state_schema.js";

/** The vocabulary is owned by the persisted schema; this module owns the mode predicates */
export { FILES_MODES, type FilesMode } from "../../shared/config/ui_state_schema.js";

/** The live feed owns the panel; the table modes share one feed builder */
export function isStreamMode(mode: FilesMode): mode is "stream" {
  return mode === "stream";
}

/**
 * The sort the file table should use. In stream mode the table is not rendered, so the
 * caller names the sort its non-stream state should fall back to.
 */
export function sortModeOf(mode: FilesMode, fallback: FileSortMode): FileSortMode {
  return isStreamMode(mode) ? fallback : mode;
}
