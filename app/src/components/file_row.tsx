// Path: app/src/components/file_row.tsx
// Description: Draggable file row with file-type icon, context menu, and star toggle

import type React from "react";
import { useCallback } from "react";
import type { FileEntry } from "../shared/protocol.js";
import { getFileFamily, FileIcon } from "../lib/icons/index.js";
import "../styles/file_row.css";

interface FileRowProps {
  file: FileEntry;
  isStarred: boolean;
  isSelected: boolean;
  onDragStart: (path: string, event: React.MouseEvent) => void | Promise<void>;
  onSelect: (path: string, event: React.MouseEvent) => void;
  onToggleStar: () => void;
  onContextMenu: (e: React.MouseEvent, file: FileEntry) => void;
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
  isStarred,
  isSelected,
  onDragStart,
  onSelect,
  onToggleStar,
  onContextMenu,
}: FileRowProps): React.JSX.Element {
  // MouseDown on row — modifier keys = selection, no modifier = drag
  const handleRowMouseDown = useCallback(
    (e: React.MouseEvent) => {
      if (e.button !== 0) return;
      if ((e.target as HTMLElement).closest("button")) return;

      if (e.shiftKey || e.metaKey || e.ctrlKey) {
        e.preventDefault(); // Prevent text selection from shift-click
        onSelect(file.path, e);
      } else {
        void onDragStart(file.path, e);
      }
    },
    [file.path, onDragStart, onSelect]
  );

  // Star button click -> toggle starred (no copy, no drag)
  const handleStarClick = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      onToggleStar();
    },
    [onToggleStar]
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
      onMouseDown={handleRowMouseDown}
      onContextMenu={handleContextMenu}
      title="Right-click for file actions"
    >
      <FileIcon family={family} />
      <div className="file-info">
        <span className="file-name">{fileName}</span>
        {directory && <span className="file-dir">{directory}</span>}
      </div>
      <span className="file-time">{formatRelativeTime(file.mtime)}</span>
      <button
        type="button"
        className={`file-star-button${isStarred ? " file-star-button--active" : ""}`}
        onClick={handleStarClick}
        title={isStarred ? "Unfavourite file" : "Favourite file"}
        aria-label={isStarred ? "Unfavourite file" : "Favourite file"}
        aria-pressed={isStarred}
      >
        ★
      </button>
    </div>
  );
}
