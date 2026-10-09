// Path: app/src/hooks/stream/use_stream_images.ts
// Description: Projects Stream cards and host visibility to the shared bounded image tile owner

import { useMemo } from "react";
import { collectTileTargets } from "../../lib/stream/stream_tile_targets.js";
import type { StreamSnapshot } from "../../lib/stream/stream_types.js";
import type { RepoRoot } from "../../shared/config.js";
import { useImageTiles, type ImageTiles } from "../images/use_image_tiles.js";

export interface StreamImageTiles extends ImageTiles {
  readonly repoId: string;
}

export function useStreamImages(repoId: string, snapshot: StreamSnapshot, repoRoot: RepoRoot | undefined): StreamImageTiles {
  const targets = useMemo(() => repoRoot === undefined
    ? []
    : collectTileTargets(repoId, snapshot.ring.cards, JSON.stringify([repoId, repoRoot.kind, repoRoot.path])),
  [repoId, snapshot.ring.cards, repoRoot]);
  const tiles = useImageTiles(targets, snapshot.visible && !snapshot.documentHidden);
  return useMemo(() => ({ repoId, byKey: tiles.byKey }), [repoId, tiles]);
}
