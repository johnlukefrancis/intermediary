// Path: app/src/components/bundles/bundle_list.tsx
// Description: Single LATEST bundle row (inline, no header)

import type React from "react";
import type { BundleInfo } from "../../shared/protocol.js";
import { BundleRow } from "./bundle_row.js";

interface BundleListProps {
  bundles: BundleInfo[];
  onDragStart: (hostPath: string) => Promise<void>;
  emptyMessage?: string;
  /** Timestamp (ms) when bundle was last built, for fresh pulse animation */
  freshlyBuiltAt?: number | null;
}

export function BundleList({
  bundles,
  onDragStart,
  emptyMessage = "No bundles yet",
  freshlyBuiltAt,
}: BundleListProps): React.JSX.Element {
  // Only show LATEST bundle (first one if exists)
  const latestBundle = bundles[0];

  if (!latestBundle) {
    const isWaiting = emptyMessage.toLowerCase().includes("waiting");
    const className = isWaiting ? "empty-state empty-state--waiting" : "empty-state";
    return (
      <div className="bundle-list empty">
        <p className={className}>{emptyMessage}</p>
      </div>
    );
  }

  return (
    <div className="bundle-list">
      <BundleRow bundle={latestBundle} onDragStart={onDragStart} freshlyBuiltAt={freshlyBuiltAt} />
    </div>
  );
}
