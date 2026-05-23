// Path: app/src/components/auto_files_row.tsx
// Description: Single Auto files table row with activity telemetry

import type React from "react";
import { useCallback, useRef } from "react";
import { getFileFamily, FileIcon } from "../lib/icons/index.js";
import type {
  FeedFileEntry,
  FileActivityGraphColumn,
  FileActivityTrend,
  FileSortMode,
} from "../lib/files/file_feed.js";

const DRAG_START_DISTANCE_PX = 6;
const ACTIVITY_GRAPH_ROWS = 6;

interface AutoFilesRowProps {
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

function activityColumnStyle(column: FileActivityGraphColumn): React.CSSProperties & {
  "--activity-strength": string;
  "--activity-roughness": string;
} {
  return {
    "--activity-strength": column.value.toFixed(3),
    "--activity-roughness": column.roughness.toFixed(3),
  };
}

function graphDotJitter(
  column: FileActivityGraphColumn,
  columnIndex: number,
  rowIndex: number
): number {
  const unit = (column.roughness * 5.13 + columnIndex * 0.173 + rowIndex * 0.311) % 1;
  return (unit - 0.5) * 0.12;
}

function graphDotThreshold(rowIndex: number): number {
  return (rowIndex + 0.55) / ACTIVITY_GRAPH_ROWS;
}

function isGraphDotLit(
  column: FileActivityGraphColumn,
  columnIndex: number,
  rowIndex: number
): boolean {
  return column.value + graphDotJitter(column, columnIndex, rowIndex) >= graphDotThreshold(rowIndex);
}

function isGraphDotEdge(column: FileActivityGraphColumn, rowIndex: number): boolean {
  return Math.abs(column.value - graphDotThreshold(rowIndex)) <= 0.12;
}

function isGraphDotRough(
  column: FileActivityGraphColumn,
  columnIndex: number,
  rowIndex: number
): boolean {
  const unit = (column.roughness * 11.37 + columnIndex * 0.271 + rowIndex * 0.419) % 1;
  return unit > 0.62;
}

export function AutoFilesRow({
  file,
  rank,
  isSelected,
  sortMode,
  onDragStart,
  onSelect,
  onOpen,
  onContextMenu,
}: AutoFilesRowProps): React.JSX.Element {
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
      className="auto-files-row"
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
      <span className="auto-files-rank">{rank.toString().padStart(2, "0")}</span>
      <div className="auto-files-file">
        <span className="auto-files-kind">
          <FileIcon family={getFileFamily(file.path)} />
          <span>{getExtensionLabel(file.path)}</span>
        </span>
        <span className="auto-files-copy">
          <span className="auto-files-name">{getFileName(file.path)}</span>
          {directory && <span className="auto-files-dir">{directory}</span>}
        </span>
      </div>
      <div className="auto-files-activity" aria-label="Recent activity graph">
        {file.activityGraph.map((column, index) => (
          <span
            key={index}
            className="auto-files-activity-column"
            data-band={column.band}
            data-lit={column.value >= 0.08 || undefined}
            style={activityColumnStyle(column)}
          >
            {Array.from({ length: ACTIVITY_GRAPH_ROWS }, (_, rowIndex) => {
              const isLit = isGraphDotLit(column, index, rowIndex);
              const isEdge = isLit && isGraphDotEdge(column, rowIndex);
              const isRough = isLit && isGraphDotRough(column, index, rowIndex);
              return (
                <span
                  key={rowIndex}
                  className="auto-files-activity-dot"
                  data-lit={isLit || undefined}
                  data-edge={isEdge || undefined}
                  data-rough={isRough || undefined}
                />
              );
            })}
          </span>
        ))}
      </div>
      <span
        className="auto-files-trend"
        data-trend={file.trend}
        aria-label={trendLabel(file.trend)}
      >
        {file.trend === "up" ? "↑" : file.trend === "down" ? "↓" : "–"}
      </span>
      <span className="auto-files-last">{formatRelativeTime(file.activity.lastSeenAtIso)}</span>
      <span className="auto-files-count">{file.activity.updateCount}</span>
      <div className="auto-files-pulse" aria-label="24-hour pulse">
        {file.pulse.map((count, index) => (
          <span
            key={index}
            className="auto-files-pulse-dot"
            style={pulseStyle(count, pulseMax)}
            data-lit={count > 0 || undefined}
          />
        ))}
      </div>
    </div>
  );
}
