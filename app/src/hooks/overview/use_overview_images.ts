// Path: app/src/hooks/overview/use_overview_images.ts
// Description: Visible Overview image metadata projected into the shared revision-bound tile loader

import { useMemo } from "react";
import type { OverviewFeedEntry } from "../../components/overview/overview_feed_entries.js";
import type { TileTarget } from "../../lib/images/image_tile_targets.js";
import { previewableImage } from "../../lib/images/image_tile_pixels.js";
import { useImageTiles, type ImageTiles } from "../images/use_image_tiles.js";

export function useOverviewImages(entries: readonly OverviewFeedEntry[], documentHidden: boolean): ImageTiles {
  const targets = useMemo(() => {
    const projected: TileTarget[] = [];
    for (const entry of entries) {
      if (entry.kind !== "edit") continue;
      const { repo, edit } = entry;
      const image = edit.image;
      if (image === null) continue;
      projected.push({
        key: entry.key,
        sourceKey: JSON.stringify([repo.repoId, repo.root.kind, repo.root.path, edit.path]),
        repoId: repo.repoId, path: edit.path, bytes: image.bytes, mtimeMs: image.mtimeMs,
        fetchable: edit.op !== "remove" && previewableImage(edit.path, image.mimeType, image.bytes),
        stamp: edit.at,
      });
    }
    return projected;
  }, [entries]);
  return useImageTiles(targets, !documentHidden);
}
