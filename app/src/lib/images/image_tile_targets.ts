// Path: app/src/lib/images/image_tile_targets.ts
// Description: Image tile source identities, newest-thumbnail retention and source-scoped BEFORE pairing

import { MAX_IMAGE_TILES } from "./image_tile_bounds.js";

export interface TileTarget {
  key: string;
  /** Exact repository/root/path identity supplied by the view projection. */
  sourceKey: string;
  repoId: string;
  path: string;
  bytes: number;
  mtimeMs: number;
  fetchable: boolean;
  stamp: number;
}

/** Targets arrive in reading order, oldest first; unfetchable slots consume no budget. */
export function retainedKeys(targets: readonly TileTarget[]): ReadonlySet<string> {
  const kept = new Set<string>();
  for (let index = targets.length - 1; index >= 0; index -= 1) {
    const target = targets[index];
    if (target === undefined || !target.fetchable) continue;
    if (kept.size >= MAX_IMAGE_TILES) break;
    kept.add(target.key);
  }
  return kept;
}

/** Nearest earlier target for the same source; paths in different roots never pair. */
export function beforeKeys(targets: readonly TileTarget[]): readonly (string | null)[] {
  const lastBySource = new Map<string, string>();
  return targets.map((target) => {
    const before = lastBySource.get(target.sourceKey) ?? null;
    lastBySource.set(target.sourceKey, target.key);
    return before;
  });
}
