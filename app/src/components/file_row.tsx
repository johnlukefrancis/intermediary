// Path: app/src/components/file_row.tsx
// Description: Draggable file row with click-to-copy, drag handle, and star toggle

import type React from "react";
import { useCallback } from "react";
import type { FileEntry, StagedInfo } from "../shared/protocol.js";
import "../styles/file_row.css";

interface FileRowProps {
  file: FileEntry;
  repoId: string;
  stagedInfo: StagedInfo | undefined;
  isStarred: boolean;
  onDragStart: (
    repoId: string,
    relativePath: string,
    stagedInfo: StagedInfo | undefined
  ) => void | Promise<void>;
  onToggleStar: () => void;
}

function formatRelativeTime(isoDate: string): string {
  if (!isoDate) return "—";

  const then = new Date(isoDate).getTime();
  if (Number.isNaN(then)) return "—";

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
  repoId,
  stagedInfo,
  isStarred,
  onDragStart,
  onToggleStar,
}: FileRowProps): React.JSX.Element {
  const clipboardText = `@${file.path}`;

  // Click row -> copy @path to clipboard (unless click originated from a button)
  const handleRowClick = useCallback(
    (e: React.MouseEvent) => {
      if ((e.target as HTMLElement).closest("button")) return;
      void navigator.clipboard.writeText(clipboardText);
    },
    [clipboardText]
  );

  // Drag handle mousedown -> copy @path + trigger drag
  const handleDragHandleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      if (e.button !== 0) return;
      e.stopPropagation();
      void navigator.clipboard.writeText(clipboardText);
      void onDragStart(repoId, file.path, stagedInfo);
    },
    [repoId, file.path, stagedInfo, clipboardText, onDragStart]
  );

  // Star button click -> toggle starred (no copy, no drag)
  const handleStarClick = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      onToggleStar();
    },
    [onToggleStar]
  );

  const fileName = getFileName(file.path);
  const directory = getDirectory(file.path);

  return (
    <div
      className="file-row"
      onClick={handleRowClick}
      title={`Click to copy ${clipboardText}`}
    >
      <span className={`file-state-marker file-state-marker--${file.changeType}`} />
      <button
        type="button"
        className="file-drag-handle"
        onMouseDown={handleDragHandleMouseDown}
        title="Drag to share (also copies path)"
        aria-label="Drag file"
      >
        ⋮⋮
      </button>
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
