// Path: app/src/hooks/stream/stream_tile_records.ts
// Description: The panel's tile pixel records: statuses, Blob release, the decoded-pixel lookup, and the published projection

import { beforeKeys, type TileTarget } from "../../lib/stream/stream_tile_targets.js";

/**
 * "dropped" is a tile released to stay inside the retention budgets; "superseded" a read that came
 * back for another revision than the card announced, or that the agent refused because the file was
 * rewritten under it; "tooLarge" a bitmap past MAX_TILE_PIXELS.
 * Each keeps its slot and never refetches until the tile is replaced in place.
 */
export type StreamTileStatus = "idle" | "loading" | "ready" | "dropped" | "error" | "superseded" | "tooLarge";

/** How a read ended; `ready` is the only outcome that keeps a Blob */
export type StreamTileOutcome = Exclude<StreamTileStatus, "idle" | "loading" | "dropped">;

export interface StreamImageTile {
  status: StreamTileStatus;
  /** This slot's own Blob URL, alive exactly as long as the tile is retained */
  url: string | null;
  width: number;
  height: number;
  /** The pixels this path showed before its newest edit: the BEFORE half of an expanded pair */
  beforeUrl: string | null;
}

/** The panel's tile set: `byKey` is keyed by `tileKey(repoId, cardId, path)` (stream_tile_targets.ts) */
export interface StreamImageTiles {
  readonly repoId: string;
  readonly byKey: ReadonlyMap<string, StreamImageTile>;
}

export interface TileRecord {
  status: StreamTileStatus;
  url: string | null;
  /** The Blob a replaced-in-place tile showed before its refetch; revoked with the record */
  previousUrl: string | null;
  /** Decoded size; kept after a release so the pixel budget still charges the slot and never re-admits it */
  width: number;
  height: number;
  /** The tile's updatedAtMs the pixels were read for; a newer stamp means a refetch */
  stamp: number;
}

export function emptyTiles(repoId: string): StreamImageTiles {
  return { repoId, byKey: new Map<string, StreamImageTile>() };
}

export function revoke(url: string | null): void {
  if (url !== null) URL.revokeObjectURL(url);
}

export function release(record: TileRecord): void {
  revoke(record.url);
  revoke(record.previousUrl);
  record.url = null;
  record.previousUrl = null;
}

/** Bitmap pixels a record holds or held; the retention walk charges them against MAX_RETAINED_PIXELS */
export function decodedPixelsOf(records: ReadonlyMap<string, TileRecord>): (key: string) => number {
  return (key) => {
    const record = records.get(key);
    return record === undefined ? 0 : record.width * record.height;
  };
}

function sameTile(a: StreamImageTile, b: StreamImageTile): boolean {
  return (
    a.status === b.status &&
    a.url === b.url &&
    a.width === b.width &&
    a.height === b.height &&
    a.beforeUrl === b.beforeUrl
  );
}

/** The next published set over `targets`, reusing `carried` entries that did not move; null when nothing changed */
export function projectTiles(
  repoId: string,
  targets: readonly TileTarget[],
  records: ReadonlyMap<string, TileRecord>,
  carried: StreamImageTiles
): StreamImageTiles | null {
  const next = new Map<string, StreamImageTile>();
  const befores = beforeKeys(targets);
  let changed = false;
  for (const [index, target] of targets.entries()) {
    const record = records.get(target.key);
    const beforeKey = befores[index] ?? null;
    const before = beforeKey === null ? undefined : records.get(beforeKey);
    const candidate: StreamImageTile = {
      status: record?.status ?? "idle",
      url: record?.url ?? null,
      width: record?.width ?? 0,
      height: record?.height ?? 0,
      beforeUrl: record?.previousUrl ?? (before?.status === "ready" ? before.url : null),
    };
    const kept = carried.byKey.get(target.key);
    if (kept !== undefined && sameTile(kept, candidate)) {
      next.set(target.key, kept);
    } else {
      next.set(target.key, candidate);
      changed = true;
    }
  }
  if (!changed && carried.repoId === repoId && next.size === carried.byKey.size) return null;
  return { repoId, byKey: next };
}
