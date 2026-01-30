// Path: app/src/components/bundles/bundle_list.tsx
// Description: List of built bundles with drag handles

import type React from "react";
import type { BundleInfo } from "../../shared/protocol.js";
import { BundleRow } from "./bundle_row.js";

interface BundleListProps {
  bundles: BundleInfo[];
  onDragStart: (windowsPath: string) => Promise<void>;
  emptyMessage?: string;
}

export function BundleList({
  bundles,
  onDragStart,
  emptyMessage = "No bundles yet",
}: BundleListProps): React.JSX.Element {
  if (bundles.length === 0) {
    return (
      <div className="bundle-list empty">
        <p className="bundle-list-empty">{emptyMessage}</p>
      </div>
    );
  }

  return (
    <div className="bundle-list">
      <div className="bundle-list-header">Built Bundles</div>
      <div className="bundle-list-items">
        {bundles.map((bundle) => (
          <BundleRow
            key={bundle.windowsPath}
            bundle={bundle}
            onDragStart={onDragStart}
          />
        ))}
      </div>
    </div>
  );
}
