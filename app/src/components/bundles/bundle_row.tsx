// Path: app/src/components/bundles/bundle_row.tsx
// Description: Individual bundle row with drag support

import type React from "react";
import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useConfig } from "../../hooks/use_config.js";
import type { BundleInfo } from "../../shared/protocol.js";

/** Duration (ms) for the "freshly built" pulse animation */
const FRESH_DURATION_MS = 5000;

interface BundleRowProps {
  bundle: BundleInfo;
  onDragStart: (hostPath: string) => Promise<void>;
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
  const {
    config: { agentDistro },
  } = useConfig();
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

  const startBundleDrag = useCallback(
    async () => {
      // Copy context text to clipboard for pasting after drop
      const contextText = `Latest bundle: ${bundle.fileName}`;
      void navigator.clipboard.writeText(contextText);

      setIsDragging(true);
      try {
        await onDragStart(bundle.hostPath);
      } finally {
        setIsDragging(false);
      }
    },
    [bundle.hostPath, bundle.fileName, onDragStart]
  );

  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      if (e.button !== 0) return;
      void startBundleDrag();
    },
    [startBundleDrag]
  );

  const handleDownloadMouseDown = useCallback(
    (e: React.MouseEvent<HTMLButtonElement>) => {
      e.stopPropagation();
    },
    []
  );

  const handleDownloadClick = useCallback(
    (e: React.MouseEvent<HTMLButtonElement>) => {
      e.stopPropagation();
      void invoke("reveal_host_file_in_file_manager", {
        hostPath: bundle.hostPath,
        distroOverride: agentDistro,
      }).catch((err: unknown) => {
        console.error("[BundleRow] Failed to reveal bundle:", err);
      });
    },
    [agentDistro, bundle.hostPath]
  );

  const className = [
    "bundle-row",
    isDragging && "dragging",
    isFresh && "bundle-row--fresh",
  ].filter(Boolean).join(" ");

  return (
    <div
      className={className}
      onMouseDown={handleMouseDown}
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
      <button
        type="button"
        className="bundle-download-button"
        title="Open bundle location"
        aria-label={`Open ${bundle.fileName} location`}
        onMouseDown={handleDownloadMouseDown}
        onClick={handleDownloadClick}
      >
        <DownloadIcon />
      </button>
    </div>
  );
}

function DownloadIcon(): React.JSX.Element {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M12 4v10" />
      <path d="M7 10l5 5 5-5" />
      <path d="M5 20h14" />
    </svg>
  );
}
