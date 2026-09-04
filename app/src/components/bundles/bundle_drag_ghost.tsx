// Path: app/src/components/bundles/bundle_drag_ghost.tsx
// Description: Floating label that follows the pointer during an in-tree row drag

import type React from "react";
import { createPortal } from "react-dom";
import { baseName } from "../../lib/bundles/bundle_selection_visibility.js";

interface BundleDragGhostProps {
  paths: readonly string[];
  position: { x: number; y: number } | null;
}

export function BundleDragGhost({ paths, position }: BundleDragGhostProps): React.JSX.Element | null {
  if (position === null || paths.length === 0) return null;
  const label = paths.length === 1 ? baseName(paths[0] as string) : `${paths.length} items`;

  return createPortal(
    <div
      className="bundle-drag-ghost"
      style={{ left: position.x, top: position.y }}
    >
      {label}
    </div>,
    document.body
  );
}
