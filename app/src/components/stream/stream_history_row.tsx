// Path: app/src/components/stream/stream_history_row.tsx
// Description: Compact scrollback row seeding the Stream panel from the existing recent list

import type React from "react";
import { getFileFamily, FileIcon } from "../../lib/icons/index.js";
import { formatRelativeTime } from "../../lib/files/relative_time.js";
import type { VisibleFileKind } from "../../lib/files/file_feed.js";

interface StreamHistoryRowProps {
  id: number;
  path: string;
  fileKind: VisibleFileKind;
  lastSeenAtIso: string;
  /** Hidden by the type filter; the row stays mounted */
  filtered: boolean;
  /** Roving focus: 0 for the current row, -1 otherwise */
  tabIndex: number;
  onFocus: (id: number) => void;
  onOpen: () => void;
  onContextMenu: (event: React.MouseEvent, path: string) => void;
}

function fileName(path: string): string {
  return path.split("/").at(-1) ?? path;
}

function directory(path: string): string {
  const index = path.lastIndexOf("/");
  return index === -1 ? "" : path.slice(0, index);
}

/**
 * Scrollback, not a card: no content, no arrival animation, muted throughout, so the live
 * cards that land beneath it read as the present.
 */
export function StreamHistoryRow({
  id,
  path,
  fileKind,
  lastSeenAtIso,
  filtered,
  tabIndex,
  onFocus,
  onOpen,
  onContextMenu,
}: StreamHistoryRowProps): React.JSX.Element {
  const dir = directory(path);

  return (
    <div
      className="stream-history-row"
      data-stream-id={id}
      data-file-kind={fileKind}
      data-filtered={filtered || undefined}
      tabIndex={tabIndex}
      onFocus={() => { onFocus(id); }}
      onDoubleClick={onOpen}
      onContextMenu={(event) => {
        event.preventDefault();
        onContextMenu(event, path);
      }}
      title="Double-click to preview supported files; right-click for file actions"
    >
      <span className="stream-history-row__icon">
        <FileIcon family={getFileFamily(path)} />
      </span>
      <span className="stream-history-row__path">
        <span className="stream-history-row__name">{fileName(path)}</span>
        {dir && <span className="stream-history-row__dir">{dir}</span>}
      </span>
      <span className="stream-history-row__time">{formatRelativeTime(lastSeenAtIso)}</span>
    </div>
  );
}
