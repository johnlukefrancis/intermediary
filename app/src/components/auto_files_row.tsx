// Path: app/src/components/auto_files_row.tsx
// Description: Single Auto files table row with activity telemetry

import type React from "react";
import { useCallback } from "react";
import { getFileFamily, FileIcon } from "../lib/icons/index.js";
import { AutoFilesActivityStack } from "./auto_files_activity_stack.js";
import { useDragOutPointer } from "../hooks/use_drag_out_pointer.js";
import { formatRelativeTime } from "../lib/files/relative_time.js";
import type { FeedFileEntry, FileSortMode } from "../lib/files/file_feed.js";

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
  const handleDragStart = useCallback(() => onDragStart(file.path), [file.path, onDragStart]);
  const handleSelect = useCallback(
    (event: Pick<React.MouseEvent, "ctrlKey" | "metaKey" | "shiftKey">) => {
      onSelect(file.path, event);
    },
    [file.path, onSelect]
  );
  const pointer = useDragOutPointer({ onDragStart: handleDragStart, onSelect: handleSelect });

  const directory = getDirectory(file.path);

  return (
    <div
      className="auto-files-row"
      data-selected={isSelected || undefined}
      data-activity={file.activityBadge ?? undefined}
      data-emphasis={sortMode}
      {...pointer}
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
      <span className="auto-files-last">{formatRelativeTime(file.activity.lastSeenAtIso)}</span>
      <span className="auto-files-count">{file.activity.updateCount}</span>
      <AutoFilesActivityStack activityGraph={file.activityGraph} pulse={file.pulse} />
    </div>
  );
}
