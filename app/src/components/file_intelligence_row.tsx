// Path: app/src/components/file_intelligence_row.tsx
// Description: Single File Intelligence table row with activity telemetry

import type React from "react";
import { useCallback, useRef } from "react";
import { getFileFamily, FileIcon } from "../lib/icons/index.js";
import type {
  FeedFileEntry,
  FileActivityTrend,
  FileSortMode,
} from "../lib/files/file_feed.js";

const DRAG_START_DISTANCE_PX = 6;

interface FileIntelligenceRowProps {
  file: FeedFileEntry;
  rank: number;
  isSelected: boolean;
  sortMode: FileSortMode;
  onDragStart: (path: string) => void | Promise<void>;
  onSelect: (
    path: string,
    event: Pick<React.MouseEvent, "ctrlKey" | "metaKey" | "shiftKey">
  ) => void;
  onOpen: (path: string) => void;
  onContextMenu: (e: React.MouseEvent, file: FeedFileEntry) => void;
}

function formatRelativeTime(isoDate: string): string {
  const then = new Date(isoDate).getTime();
  if (Number.isNaN(then)) return "--";

  const diffSec = Math.max(0, Math.floor((Date.now() - then) / 1000));
  if (diffSec < 60) return `${diffSec}s ago`;
  const diffMin = Math.floor(diffSec / 60);
  if (diffMin < 60) return `${diffMin}m ago`;
  const diffHour = Math.floor(diffMin / 60);
  if (diffHour < 24) return `${diffHour}h ago`;
  return `${Math.floor(diffHour / 24)}d ago`;
}

function getFileName(path: string): string {
  return path.split("/").at(-1) ?? path;
}

function getDirectory(path: string): string {
  const index = path.lastIndexOf("/");
  return index === -1 ? "" : path.slice(0, index);
}

function getExtensionLabel(path: string): string {
  const fileName = getFileName(path);
  const extension = fileName.includes(".") ? fileName.split(".").at(-1) : "";
  return extension ? extension.slice(0, 4).toUpperCase() : "FILE";
}

function trendLabel(trend: FileActivityTrend): string {
  if (trend === "up") return "Increasing activity";
  if (trend === "down") return "Cooling activity";
  return "Stable activity";
}

function pulseStyle(count: number, pulseMax: number): React.CSSProperties & {
  "--pulse-alpha": string;
} {
  return { "--pulse-alpha": String(0.22 + (count / pulseMax) * 0.78) };
}

export function FileIntelligenceRow({
  file,
  rank,
  isSelected,
  sortMode,
  onDragStart,
  onSelect,
  onOpen,
  onContextMenu,
}: FileIntelligenceRowProps): React.JSX.Element {
  const dragStartRef = useRef<{ pointerId: number; x: number; y: number } | null>(null);

  const clearPointerCapture = useCallback((target: Element, pointerId: number): void => {
    if (!(target instanceof HTMLElement) || !target.hasPointerCapture(pointerId)) return;
    target.releasePointerCapture(pointerId);
  }, []);

  const handlePointerDown = useCallback(
    (event: React.PointerEvent) => {
      if (event.button !== 0) return;
      if ((event.target as HTMLElement).closest("button")) return;

      if (event.shiftKey || event.metaKey || event.ctrlKey) {
        event.preventDefault();
        onSelect(file.path, event);
        return;
      }

      dragStartRef.current = {
        pointerId: event.pointerId,
        x: event.clientX,
        y: event.clientY,
      };
      event.currentTarget.setPointerCapture(event.pointerId);
    },
    [file.path, onSelect]
  );

  const handlePointerMove = useCallback(
    (event: React.PointerEvent) => {
      const start = dragStartRef.current;
      if (!start || start.pointerId !== event.pointerId) return;
      if ((event.buttons & 1) !== 1) {
        clearPointerCapture(event.currentTarget, event.pointerId);
        dragStartRef.current = null;
        return;
      }

      const distance = Math.hypot(event.clientX - start.x, event.clientY - start.y);
      if (distance < DRAG_START_DISTANCE_PX) return;

      clearPointerCapture(event.currentTarget, event.pointerId);
      dragStartRef.current = null;
      void onDragStart(file.path);
    },
    [clearPointerCapture, file.path, onDragStart]
  );

  const handlePointerEnd = useCallback(
    (event: React.PointerEvent) => {
      clearPointerCapture(event.currentTarget, event.pointerId);
      if (dragStartRef.current?.pointerId === event.pointerId) {
        dragStartRef.current = null;
      }
    },
    [clearPointerCapture]
  );

  const directory = getDirectory(file.path);
  const pulseMax = Math.max(...file.pulse, 1);

  return (
    <div
      className="file-intelligence-row"
      data-selected={isSelected || undefined}
      data-activity={file.activityBadge ?? undefined}
      data-emphasis={sortMode}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerEnd}
      onPointerCancel={handlePointerEnd}
      onDoubleClick={() => { onOpen(file.path); }}
      onContextMenu={(event) => {
        event.preventDefault();
        onContextMenu(event, file);
      }}
      title="Double-click to preview supported files; drag to stage for handoff; right-click for file actions"
    >
      <span className="file-intelligence-rank">{rank.toString().padStart(2, "0")}</span>
      <div className="file-intelligence-file">
        <span className="file-intelligence-kind">
          <FileIcon family={getFileFamily(file.path)} />
          <span>{getExtensionLabel(file.path)}</span>
        </span>
        <span className="file-intelligence-copy">
          <span className="file-intelligence-name">{getFileName(file.path)}</span>
          {directory && <span className="file-intelligence-dir">{directory}</span>}
        </span>
      </div>
      <div className="file-intelligence-activity" aria-label="Activity intensity">
        {Array.from({ length: 10 }, (_, index) => (
          <span
            key={index}
            className="file-intelligence-activity-cell"
            data-lit={index < file.activityBlocks || undefined}
            data-level={index}
          />
        ))}
      </div>
      <span
        className="file-intelligence-trend"
        data-trend={file.trend}
        aria-label={trendLabel(file.trend)}
      >
        {file.trend === "up" ? "↑" : file.trend === "down" ? "↓" : "–"}
      </span>
      <span className="file-intelligence-last">{formatRelativeTime(file.activity.lastSeenAtIso)}</span>
      <span className="file-intelligence-count">{file.activity.updateCount}</span>
      <div className="file-intelligence-pulse" aria-label="24-hour pulse">
        {file.pulse.map((count, index) => (
          <span
            key={index}
            className="file-intelligence-pulse-dot"
            style={pulseStyle(count, pulseMax)}
            data-lit={count > 0 || undefined}
          />
        ))}
      </div>
    </div>
  );
}
