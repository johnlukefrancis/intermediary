// Path: app/src/lib/stream/stream_tile_targets.ts
// Description: Projects Stream strip slots and repository-scoped source identities into shared image targets

import { previewableImage } from "../images/image_tile_pixels.js";
import type { TileTarget } from "../images/image_tile_targets.js";
import type { StreamRingCard } from "./stream_types.js";

export function tileKey(repoId: string, cardId: number, path: string): string {
  return `${repoId}:${String(cardId)}:${path}`;
}

/** Ring order, then tile order: the reading order of every tile slot the panel shows */
export function collectTileTargets(repoId: string, cards: readonly StreamRingCard[], sourceKeyPrefix: string): TileTarget[] {
  const targets: TileTarget[] = [];
  for (const card of cards) {
    if (card.kind !== "images") continue;
    for (const tile of card.tiles) {
      const body = tile.body;
      const live = body.status === "image" && tile.op !== "remove";
      targets.push({
        key: tileKey(repoId, card.id, tile.path),
        sourceKey: JSON.stringify([sourceKeyPrefix, tile.path]),
        repoId,
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
