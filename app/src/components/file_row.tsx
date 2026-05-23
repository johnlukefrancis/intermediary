// Path: app/src/components/file_row.tsx
// Description: Draggable file row with file-type icon, context menu, and activity heat

import type React from "react";
import { useCallback, useRef } from "react";
import { getFileFamily, FileIcon } from "../lib/icons/index.js";
import type { FeedFileEntry, FileActivityBadge } from "../lib/files/file_feed.js";
import "../styles/file_row.css";

const DRAG_START_DISTANCE_PX = 6;

interface FileRowProps {
  file: FeedFileEntry;
  isSelected: boolean;
  activityBadge: FileActivityBadge | null;
  onDragStart: (path: string) => void | Promise<void>;
  onSelect: (
    path: string,
    event: Pick<React.MouseEvent, "ctrlKey" | "metaKey" | "shiftKey">
  ) => void;
  onOpen: (path: string) => void;
  onContextMenu: (e: React.MouseEvent, file: FeedFileEntry) => void;
}

function formatRelativeTime(isoDate: string): string {
  if (!isoDate) return "\u2014";

  const then = new Date(isoDate).getTime();
  if (Number.isNaN(then)) return "\u2014";

  const now = Date.now();
  const diffMs = now - then;
  const diffSec = Math.floor(diffMs / 1000);

  if (diffSec < 60) return `${diffSec}s ago`;
  const diffMin = Math.floor(diffSec / 60);
  if (diffMin < 60) return `${diffMin}m ago`;
  const diffHour = Math.floor(diffMin / 60);
  if (diffHour < 24) return `${diffHour}h ago`;
  const diffDay = Math.floor(diffHour / 24);
  return `${diffDay}d ago`;
}

function getFileName(path: string): string {
  const parts = path.split("/");
  return parts[parts.length - 1] ?? path;
}

function getDirectory(path: string): string {
  const parts = path.split("/");
  if (parts.length <= 1) return "";
  return parts.slice(0, -1).join("/");
}

export function FileRow({
  file,
  isSelected,
  activityBadge,
  onDragStart,
  onSelect,
  onOpen,
  onContextMenu,
}: FileRowProps): React.JSX.Element {
  const dragStartRef = useRef<{
    pointerId: number;
    x: number;
    y: number;
  } | null>(null);

  const clearPointerCapture = useCallback((target: Element, pointerId: number): void => {
    if (!(target instanceof HTMLElement) || !target.hasPointerCapture(pointerId)) return;
    target.releasePointerCapture(pointerId);
  }, []);

  const handleRowPointerDown = useCallback(
    (e: React.PointerEvent) => {
      if (e.button !== 0) return;
      if ((e.target as HTMLElement).closest("button")) return;

      if (e.shiftKey || e.metaKey || e.ctrlKey) {
        e.preventDefault(); // Prevent text selection from shift-click
        onSelect(file.path, e);
      } else {
        dragStartRef.current = {
          pointerId: e.pointerId,
          x: e.clientX,
          y: e.clientY,
        };
        e.currentTarget.setPointerCapture(e.pointerId);
      }
    },
    [file.path, onSelect]
  );

  const handleRowPointerMove = useCallback(
    (e: React.PointerEvent) => {
      const start = dragStartRef.current;
      if (!start || start.pointerId !== e.pointerId) return;
      if ((e.buttons & 1) !== 1) {
        clearPointerCapture(e.currentTarget, e.pointerId);
        dragStartRef.current = null;
        return;
      }

      const deltaX = e.clientX - start.x;
      const deltaY = e.clientY - start.y;
      const distance = Math.hypot(deltaX, deltaY);
      if (distance < DRAG_START_DISTANCE_PX) return;

      clearPointerCapture(e.currentTarget, e.pointerId);
      dragStartRef.current = null;
      void onDragStart(file.path);
    },
    [clearPointerCapture, file.path, onDragStart]
  );

  const handleRowPointerEnd = useCallback(
    (e: React.PointerEvent) => {
      clearPointerCapture(e.currentTarget, e.pointerId);
      if (dragStartRef.current?.pointerId === e.pointerId) {
        dragStartRef.current = null;
      }
    },
    [clearPointerCapture]
  );

  const handleDoubleClick = useCallback(
    (e: React.MouseEvent) => {
      if ((e.target as HTMLElement).closest("button")) return;
      e.preventDefault();
      onOpen(file.path);
    },
    [file.path, onOpen]
  );

  // Right-click -> context menu
  const handleContextMenu = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      onContextMenu(e, file);
    },
    [onContextMenu, file]
  );

  const fileName = getFileName(file.path);
  const directory = getDirectory(file.path);
  const family = getFileFamily(file.path);

  return (
    <div
      className="file-row"
      data-change-type={file.changeType}
      data-selected={isSelected || undefined}
      data-activity={activityBadge ?? undefined}
      onPointerDown={handleRowPointerDown}
      onPointerMove={handleRowPointerMove}
      onPointerUp={handleRowPointerEnd}
      onPointerCancel={handleRowPointerEnd}
      onDoubleClick={handleDoubleClick}
      onContextMenu={handleContextMenu}
      title="Double-click to preview supported files; drag to stage for handoff; right-click for file actions"
    >
      <FileIcon family={family} />
      <div className="file-info">
        <span className="file-name">{fileName}</span>
        {directory && <span className="file-dir">{directory}</span>}
      </div>
      <span className="file-time">{formatRelativeTime(file.mtime)}</span>
      <span className="file-heat-badge" aria-hidden="true" />
    </div>
  );
}
