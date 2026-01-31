// Path: app/src/components/file_row.tsx
// Description: Draggable file row with state marker and timestamp

import type React from "react";
import { useCallback } from "react";
import type { FileEntry, StagedInfo } from "../shared/protocol.js";
import "../styles/file_row.css";

interface FileRowProps {
  file: FileEntry;
  repoId: string;
  stagedInfo: StagedInfo | undefined;
  onDragStart: (
    repoId: string,
    relativePath: string,
    stagedInfo: StagedInfo | undefined
  ) => void | Promise<void>;
}

function formatRelativeTime(isoDate: string): string {
  const now = Date.now();
  const then = new Date(isoDate).getTime();
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
  onDragStart,
}: FileRowProps): React.JSX.Element {
  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      // Only trigger on primary button
      if (e.button !== 0) return;
      void onDragStart(repoId, file.path, stagedInfo);
    },
    [repoId, file.path, stagedInfo, onDragStart]
  );

  const fileName = getFileName(file.path);
  const directory = getDirectory(file.path);

  return (
    <div className="file-row" onMouseDown={handleMouseDown} title="Drag to share">
      <span className={`file-state-marker file-state-marker--${file.changeType}`} />
      <div className="file-info">
        <span className="file-name">{fileName}</span>
        {directory && <span className="file-dir">{directory}</span>}
      </div>
      <span className="file-time">{formatRelativeTime(file.mtime)}</span>
    </div>
  );
}
