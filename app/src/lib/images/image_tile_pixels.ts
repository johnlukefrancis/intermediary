// Path: app/src/lib/images/image_tile_pixels.ts
// Description: Shared image preview eligibility, exact revision acceptance, source pixel gate and thumbnail fit

import { AgentResponseError } from "../agent/error_codes.js";
import { isPreviewImagePath } from "../../hooks/repo_workspace_types.js";
import { IMAGE_TILE_MAX_BYTES, MAX_TILE_PIXELS, IMAGE_THUMB_MAX_PX } from "./image_tile_bounds.js";

/** Only a whitelisted extension with a wire mime type under the byte gate fetches pixels. */
export function previewableImage(path: string, mimeType: string | null, bytes: number): boolean {
  return mimeType !== null && bytes <= IMAGE_TILE_MAX_BYTES && isPreviewImagePath(path);
}

/** The revision a tile announced, and the revision a readImageFile result reports */
export interface TileRevision {
  bytes: number;
  mtimeMs: number;
}

/** Accept only the announced byte count and mtime; newer pixels cannot belong to an older tile. */
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

/** Fit the long edge to IMAGE_THUMB_MAX_PX; smaller icons and animated GIFs remain unchanged. */
export function thumbnailSize(width: number, height: number): ThumbnailSize | null {
  const edge = Math.max(width, height);
  if (edge <= IMAGE_THUMB_MAX_PX) return null;
  const scale = IMAGE_THUMB_MAX_PX / edge;
  return { width: Math.max(1, Math.round(width * scale)), height: Math.max(1, Math.round(height * scale)) };
}

/** The agent's mid-read rewrite refusal means superseded pixels; other refusals remain errors. */
export function readRefusedAsChanged(error: unknown): boolean {
  if (!(error instanceof AgentResponseError) || error.code !== "UNSUPPORTED_IMAGE_FILE") return false;
  return error.serverMessage.toLowerCase().includes("changed while");
}
