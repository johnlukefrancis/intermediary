// Path: app/src/lib/stream/stream_tile_pixels.ts
// Description: Pure pixel-acceptance rules for a strip tile: the revision a read must match, the mid-read rewrite refusal, and the decoded-size gate

import { AgentResponseError } from "../agent/error_codes.js";
import { MAX_TILE_PIXELS } from "./stream_bounds.js";

/** The revision a tile announced, and the revision a readImageFile result reports */
export interface TileRevision {
  bytes: number;
  mtimeMs: number;
}

/**
 * Pixels are shown only for the exact revision the card announced: the same byte count and the
 * same mtime. Anything else is a newer (or older) file than the card describes and is refused, so
 * newer pixels never sit under an older card; a replace-in-place tile refetches for its own revision.
 */
export function sameRevision(tile: TileRevision, read: TileRevision): boolean {
  return tile.bytes === read.bytes && tile.mtimeMs === read.mtimeMs;
}

/** A decoded bitmap past MAX_TILE_PIXELS is released rather than held: its RGBA would dwarf the byte budget */
export function exceedsTilePixels(width: number, height: number): boolean {
  return width * height > MAX_TILE_PIXELS;
}

/**
 * The agent refuses a read whose file was rewritten under it (`UNSUPPORTED_IMAGE_FILE` with a
 * "changed while" message) rather than returning bytes from two revisions. That is the same fact
 * as a revision mismatch — the tile is behind the file, not broken — so the slot reads
 * IMAGE CHANGED at its own size instead of PREVIEW FAILED, and the path's next delta brings its
 * own tile. Matched case-insensitively; every other refusal stays an error.
 */
export function readRefusedAsChanged(error: unknown): boolean {
  if (!(error instanceof AgentResponseError) || error.code !== "UNSUPPORTED_IMAGE_FILE") return false;
  return error.serverMessage.toLowerCase().includes("changed while");
}
