// Path: app/src/components/bundles/bundle_row.tsx
// Description: Individual bundle row with drag support

import type React from "react";
import { useCallback, useEffect, useState } from "react";
import type { BundleInfo } from "../../shared/protocol.js";

/** Duration (ms) for the "freshly built" pulse animation */
const FRESH_DURATION_MS = 5000;

interface BundleRowProps {
  bundle: BundleInfo;
  onDragStart: (windowsPath: string) => Promise<void>;
  /** Timestamp (ms) when bundle was last built, for fresh pulse animation */
  freshlyBuiltAt?: number | null | undefined;
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

export function BundleRow({ bundle, onDragStart, freshlyBuiltAt }: BundleRowProps): React.JSX.Element {
  const [isDragging, setIsDragging] = useState(false);
  const [isFresh, setIsFresh] = useState(false);

  // Handle fresh bundle pulse animation
  useEffect(() => {
    if (freshlyBuiltAt == null) {
      setIsFresh(false);
      return;
    }

    const elapsed = Date.now() - freshlyBuiltAt;
    if (elapsed >= FRESH_DURATION_MS) {
      setIsFresh(false);
      return;
    }

    // Still within fresh window - show animation
    setIsFresh(true);
    const remaining = FRESH_DURATION_MS - elapsed;
    const timer = setTimeout(() => { setIsFresh(false); }, remaining);
    return () => { clearTimeout(timer); };
  }, [freshlyBuiltAt]);

  const handleMouseDown = useCallback(
    async (e: React.MouseEvent) => {
      if (e.button !== 0) return;

      // Copy context text to clipboard for pasting after drop
      const contextText = `Latest bundle: ${bundle.fileName}`;
      void navigator.clipboard.writeText(contextText);

      setIsDragging(true);
      try {
        await onDragStart(bundle.windowsPath);
      } finally {
        setIsDragging(false);
      }
    },
    [bundle.windowsPath, bundle.fileName, onDragStart]
  );

  const className = [
    "bundle-row",
    isDragging && "dragging",
    isFresh && "bundle-row--fresh",
  ].filter(Boolean).join(" ");

  return (
    <div
      className={className}
      onMouseDown={(e) => void handleMouseDown(e)}
      title="Drag to share (filename copied to clipboard)"
    >
      <span className={`bundle-state-marker${bundle.isLatestAlias ? " bundle-state-marker--latest" : ""}`} />
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
