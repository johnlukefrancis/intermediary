// Path: app/src/components/overview/overview_image_preview.tsx
// Description: Fixed-height real image preview that opens the full file workspace

import type React from "react";
import type { ImageTile } from "../../hooks/images/use_image_tiles.js";
import { baseName } from "../../lib/bundles/bundle_selection_visibility.js";

const STATUS_LABELS: Readonly<Record<ImageTile["status"], string>> = {
  idle: "Awaiting preview", loading: "Loading preview", ready: "Preview unavailable",
  dropped: "Preview no longer retained", error: "Preview unavailable",
  superseded: "Image changed since this edit", tooLarge: "Image exceeds preview limit",
};

export function OverviewImagePreview({ path, tile, onOpen }: { path: string; tile: ImageTile | undefined; onOpen: () => void }): React.JSX.Element {
  const url = tile?.url;
  const filename = baseName(path);
  const status = tile?.status ?? "idle";
  return (
    <div className="overview-image">
      <button type="button" className="overview-image__canvas" draggable aria-label={`Open ${filename}`} title="Open image workspace" onClick={onOpen}>
        {url ? <img src={url} alt={`Preview of ${filename}`} draggable={false} />
          : <span className="overview-image__waiting" role="status">{STATUS_LABELS[status]}</span>}
      </button>
    </div>
  );
}
