// Path: app/src/components/bundles/bundle_row.tsx
// Description: Individual bundle row with drag support

import type React from "react";
import { useCallback, useState } from "react";
import type { BundleInfo } from "../../shared/protocol.js";

interface BundleRowProps {
  bundle: BundleInfo;
  onDragStart: (windowsPath: string) => Promise<void>;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatRelativeTime(mtimeMs: number): string {
  const now = Date.now();
  const diffMs = now - mtimeMs;
  const diffSec = Math.floor(diffMs / 1000);
  const diffMin = Math.floor(diffSec / 60);
  const diffHour = Math.floor(diffMin / 60);
  const diffDay = Math.floor(diffHour / 24);

  if (diffSec < 60) return "just now";
  if (diffMin < 60) return `${diffMin}m ago`;
  if (diffHour < 24) return `${diffHour}h ago`;
  return `${diffDay}d ago`;
}

export function BundleRow({ bundle, onDragStart }: BundleRowProps): React.JSX.Element {
  const [isDragging, setIsDragging] = useState(false);

  const handleMouseDown = useCallback(
    async (e: React.MouseEvent) => {
      if (e.button !== 0) return;
      setIsDragging(true);
      try {
        await onDragStart(bundle.windowsPath);
      } finally {
        setIsDragging(false);
      }
    },
    [bundle.windowsPath, onDragStart]
  );

  return (
    <div className={`bundle-row ${isDragging ? "dragging" : ""}`}>
      <span className={`bundle-state-marker${bundle.isLatestAlias ? " bundle-state-marker--latest" : ""}`} />
      <div
        className="bundle-drag-handle"
        onMouseDown={(e) => void handleMouseDown(e)}
        title="Drag to share"
      />
      <div className="bundle-info">
        <span className="bundle-filename">{bundle.fileName}</span>
        <span className="bundle-meta">
          <span className="bundle-size">{formatBytes(bundle.bytes)}</span>
          <span className="bundle-time">{formatRelativeTime(bundle.mtimeMs)}</span>
          {bundle.isLatestAlias && (
            <span className="badge badge--latest">latest</span>
          )}
        </span>
      </div>
    </div>
  );
}
