// Path: app/src/hooks/images/image_tile_records.ts
// Description: Shared image tile records, Blob ownership and stable published pixel projections

import { beforeKeys, type TileTarget } from "../../lib/images/image_tile_targets.js";

/** Terminal outcomes keep their slot without retry until its announced revision changes. */
export type ImageTileStatus = "idle" | "loading" | "ready" | "dropped" | "error" | "superseded" | "tooLarge";

/** How a read ended; `ready` is the only outcome that keeps a Blob */
export type ImageTileOutcome = Exclude<ImageTileStatus, "idle" | "loading" | "dropped">;

export interface ImageTile {
  status: ImageTileStatus;
  /** This slot's own thumbnail Blob URL, alive exactly as long as the tile is retained */
  url: string | null;
  /** The SOURCE's size (the title, the icon cap), never the thumbnail's; a tooLarge slot names it */
  width: number;
  height: number;
  /** The pixels this path showed before its newest edit: the BEFORE half of an expanded pair */
  beforeUrl: string | null;
}

export interface ImageTiles {
  readonly byKey: ReadonlyMap<string, ImageTile>;
}

export interface TileRecord {
  status: ImageTileStatus;
  url: string | null;
  /** The Blob a replaced-in-place tile showed before its refetch; revoked with the record */
  previousUrl: string | null;
  /** The source's size from the probe; a released record keeps it for its title */
  width: number;
  height: number;
  target: TileTarget;
}

export function emptyTiles(): ImageTiles {
  return { byKey: new Map<string, ImageTile>() };
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

function sameTile(a: ImageTile, b: ImageTile): boolean {
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
  targets: readonly TileTarget[],
  records: ReadonlyMap<string, TileRecord>,
  carried: ImageTiles
): ImageTiles | null {
  const next = new Map<string, ImageTile>();
  const befores = beforeKeys(targets);
  let changed = false;
  for (const [index, target] of targets.entries()) {
    const record = records.get(target.key);
    const beforeKey = befores[index] ?? null;
    const before = beforeKey === null ? undefined : records.get(beforeKey);
    const candidate: ImageTile = {
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
  if (!changed && next.size === carried.byKey.size) return null;
  return { byKey: next };
}
