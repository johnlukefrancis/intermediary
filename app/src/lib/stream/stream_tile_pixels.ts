// Path: app/src/lib/stream/stream_tile_pixels.ts
// Description: Pure pixel rules for a strip tile: the revision a read must match, the mid-read rewrite refusal, the decode gate, and the thumbnail fit

import { AgentResponseError } from "../agent/error_codes.js";
import { MAX_TILE_PIXELS, STRIP_THUMB_MAX_PX } from "./stream_bounds.js";

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

/** A source past MAX_TILE_PIXELS is never decoded: even transiently its RGBA would dwarf the panel */
export function exceedsTilePixels(width: number, height: number): boolean {
  return width * height > MAX_TILE_PIXELS;
}

export interface ThumbnailSize {
  width: number;
  height: number;
}

/**
 * The size the tile retains for a source of `width`×`height`: scaled so its long edge is
 * STRIP_THUMB_MAX_PX (never below one pixel a side), or null when the source already fits and is
 * kept as-is, so an icon stays pixel-exact and a small GIF keeps its frames.
 */
export function thumbnailSize(width: number, height: number): ThumbnailSize | null {
  const edge = Math.max(width, height);
  if (edge <= STRIP_THUMB_MAX_PX) return null;
  const scale = STRIP_THUMB_MAX_PX / edge;
  return { width: Math.max(1, Math.round(width * scale)), height: Math.max(1, Math.round(height * scale)) };
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
