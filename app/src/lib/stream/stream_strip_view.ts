// Path: app/src/lib/stream/stream_strip_view.ts
// Description: How an image strip's head, rail, and footer read: count label, shared directory, op tally, clock span, bytes

import { parentPath } from "../bundles/bundle_selection_visibility.js";
import type { DeltaOp } from "../../shared/protocol.js";
import type { StreamRailVariant } from "./stream_card_grammar.js";
import type { StreamStripTile } from "./stream_strip_types.js";

export function stripCountLabel(count: number): string {
  return count === 1 ? "IMAGE" : `${String(count)} IMAGES`;
}

/** Longest common directory of every tile; `…` marks tiles that fan out below it, empty means the root */
export function stripDirLabel(tiles: readonly StreamStripTile[]): string {
  const dirs = tiles.map((tile) => parentPath(tile.path));
  const first = dirs[0];
  if (first === undefined) return "";
  let common = first.split("/").filter(Boolean);
  for (const dir of dirs.slice(1)) {
    const parts = dir.split("/").filter(Boolean);
    let shared = 0;
    while (shared < common.length && shared < parts.length && common[shared] === parts[shared]) shared += 1;
    common = common.slice(0, shared);
  }
  const prefix = common.join("/");
  const uniform = dirs.every((dir) => dir === prefix);
  if (uniform) return prefix;
  return prefix === "" ? "…" : `${prefix}/…`;
}

export function stripOpTally(tiles: readonly StreamStripTile[]): Record<DeltaOp, number> {
  const tally: Record<DeltaOp, number> = { add: 0, modify: 0, remove: 0, rename: 0 };
  for (const tile of tiles) tally[tile.op] += 1;
  return tally;
}

/** The spine colour follows the tiles: all added → success, all deleted → error, anything mixed → info */
export function stripRail(tiles: readonly StreamStripTile[]): StreamRailVariant {
  if (tiles.length > 0 && tiles.every((tile) => tile.op === "add")) return "success";
  if (tiles.length > 0 && tiles.every((tile) => tile.op === "remove" || tile.body.status === "gone")) return "error";
  return "info";
}

/** `14:32:07–14:32:19`, or the single clock when every tile arrived in the same second */
export function stripClockSpan(tiles: readonly StreamStripTile[]): string {
  let first = "";
  let last = "";
  let firstMs = Number.POSITIVE_INFINITY;
  let lastMs = Number.NEGATIVE_INFINITY;
  for (const tile of tiles) {
    if (tile.arrivedAtMs < firstMs) {
      firstMs = tile.arrivedAtMs;
      first = tile.clock;
    }
    if (tile.arrivedAtMs >= lastMs) {
      lastMs = tile.arrivedAtMs;
      last = tile.clock;
    }
  }
  return first === last ? first : `${first}–${last}`;
}

/** Clock of the tile that arrived last; the head's 52 px clock column holds one stamp */
export function stripNewestClock(tiles: readonly StreamStripTile[]): string {
  let newest: StreamStripTile | undefined;
  for (const tile of tiles) {
    if (newest === undefined || tile.arrivedAtMs >= newest.arrivedAtMs) newest = tile;
  }
  return newest?.clock ?? "";
}

/** Summed wire bytes of the tiles that still have a file; deleted tiles weigh nothing */
export function stripTotalBytes(tiles: readonly StreamStripTile[]): number {
  let total = 0;
  for (const tile of tiles) {
    if (tile.body.status === "image") total += tile.body.bytes;
  }
  return total;
}
