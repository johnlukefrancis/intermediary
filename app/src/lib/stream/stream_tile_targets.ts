// Path: app/src/lib/stream/stream_tile_targets.ts
// Description: Pure tile-retention arithmetic: the flat fetch list over every strip, which keys keep pixels, and each tile's BEFORE

import { IMAGE_TILE_BYTES_BUDGET, MAX_IMAGE_TILES, MAX_RETAINED_PIXELS } from "./stream_bounds.js";
import { previewableImage } from "./stream_card_grammar.js";
import type { StreamRingCard } from "./stream_types.js";

export interface TileTarget {
  /** `${repoId}:${cardId}:${path}`: one pixel record per tile slot, never shared across repos */
  key: string;
  cardId: number;
  path: string;
  bytes: number;
  /** The revision the tile announced beside `bytes`; a read reporting any other revision is refused */
  mtimeMs: number;
  /** Whitelisted extension, wire mime, under the size gate, and not deleted: pixels may be requested */
  fetchable: boolean;
  /** The tile's updatedAtMs; a replaced-in-place tile changes it, so its stale pixels are refetched */
  stamp: number;
}

export function tileKey(repoId: string, cardId: number, path: string): string {
  return `${repoId}:${String(cardId)}:${path}`;
}

/** Ring order, then tile order: the reading order of every tile slot the panel shows */
export function collectTileTargets(repoId: string, cards: readonly StreamRingCard[]): TileTarget[] {
  const targets: TileTarget[] = [];
  for (const card of cards) {
    if (card.kind !== "images") continue;
    for (const tile of card.tiles) {
      const body = tile.body;
      const live = body.status === "image" && tile.op !== "remove";
      targets.push({
        key: tileKey(repoId, card.id, tile.path),
        cardId: card.id,
        path: tile.path,
        bytes: body.status === "image" ? body.bytes : 0,
        mtimeMs: body.status === "image" ? body.mtimeMs : 0,
        fetchable: live && previewableImage(tile.path, body.mimeType, body.bytes),
        stamp: tile.updatedAtMs,
      });
    }
  }
  return targets;
}

/** Decoded pixels a key holds (or held: a released tile keeps reporting its size so it never oscillates back in) */
export type DecodedPixels = (key: string) => number;

const NO_PIXELS: DecodedPixels = () => 0;

/**
 * Newest first, fetchable only, until MAX_IMAGE_TILES are kept, the next tile would push the
 * summed source bytes past IMAGE_TILE_BYTES_BUDGET, or its decoded pixels would push the summed
 * bitmap past MAX_RETAINED_PIXELS. Everything older keeps its slot and loses its Blob.
 */
export function retainedKeys(targets: readonly TileTarget[], decodedPixels: DecodedPixels = NO_PIXELS): ReadonlySet<string> {
  const kept = new Set<string>();
  let bytes = 0;
  let pixels = 0;
  for (let index = targets.length - 1; index >= 0; index -= 1) {
    const target = targets[index];
    if (target === undefined || !target.fetchable) continue;
    const decoded = decodedPixels(target.key);
    if (kept.size >= MAX_IMAGE_TILES || bytes + target.bytes > IMAGE_TILE_BYTES_BUDGET || pixels + decoded > MAX_RETAINED_PIXELS) break;
    kept.add(target.key);
    bytes += target.bytes;
    pixels += decoded;
  }
  return kept;
}

/** For each target, the key of the nearest EARLIER target with the same path (its BEFORE), or null */
export function beforeKeys(targets: readonly TileTarget[]): readonly (string | null)[] {
  const lastByPath = new Map<string, string>();
  return targets.map((target) => {
    const before = lastByPath.get(target.path) ?? null;
    lastByPath.set(target.path, target.key);
    return before;
  });
}
