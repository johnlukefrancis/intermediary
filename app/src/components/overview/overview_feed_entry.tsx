// Path: app/src/components/overview/overview_feed_entry.tsx
// Description: One repository-bound feed entry with file opening, context menu and drag-out interactions

import type React from "react";
import { useCallback, useRef } from "react";
import type { RepoConfig } from "../../shared/config.js";
import type { ImageTile } from "../../hooks/images/use_image_tiles.js";
import { useDragOutPointer } from "../../hooks/use_drag_out_pointer.js";
import type { OverviewFeedEntry as Entry } from "./overview_feed_entries.js";
import { OverviewAge, OverviewFileHead } from "./overview_file_head.js";
import { OverviewSettledEdit } from "./overview_settled_edit.js";

interface OverviewFeedEntryProps {
  entry: Entry;
  now: number;
  imageTile: ImageTile | undefined;
  onOpenFile: (repo: RepoConfig, path: string) => void;
  onDragFile: (repo: RepoConfig, path: string) => void;
  onContextMenu: (event: React.MouseEvent, repo: RepoConfig, path: string) => void;
}

export function OverviewFeedEntry({ entry, now, imageTile, onOpenFile, onDragFile, onContextMenu }: OverviewFeedEntryProps): React.JSX.Element {
  const path = entry.kind === "edit" ? entry.edit.path : entry.kind === "signal" ? entry.signal.path : null;
  const deleted = entry.kind === "edit" ? entry.edit.op === "remove" : entry.kind === "signal" && entry.signal.changeType === "unlink";
  const dragged = useRef(false);
  const open = useCallback(() => { if (path !== null) onOpenFile(entry.repo, path); }, [entry.repo, onOpenFile, path]);
  const drag = useCallback(() => {
    if (path === null || deleted || dragged.current) return;
    dragged.current = true;
    onDragFile(entry.repo, path);
  }, [deleted, entry.repo, onDragFile, path]);
  const pointer = useDragOutPointer({ onDragStart: drag, enabled: path !== null && !deleted });
  return (
    <div className="overview-feed-entry" data-repo-id={entry.repo.repoId} data-kind={entry.kind}
      {...pointer} onPointerDown={(event) => { dragged.current = false; pointer.onPointerDown(event); }}
      onDragStart={(event) => { event.preventDefault(); drag(); }}
      onClickCapture={(event) => { if (dragged.current) { event.preventDefault(); event.stopPropagation(); dragged.current = false; } }}
      onKeyDownCapture={() => { dragged.current = false; }}
      onDoubleClick={(event) => { if (!(event.target instanceof Element) || event.target.closest("button") === null) open(); }}
      onContextMenu={(event) => { if (path !== null) onContextMenu(event, entry.repo, path); }}>
      {entry.kind === "edit" ? <OverviewSettledEdit edit={entry.edit} now={now} imageTile={imageTile} onOpen={open} />
        : entry.kind === "signal" ? <OverviewFileHead path={entry.signal.path} onOpen={open}
          op={entry.signal.source === "history" ? null : entry.signal.changeType === "add" ? "add" : entry.signal.changeType === "unlink" ? "remove" : "modify"}
          age={<OverviewAge at={entry.at} now={now} />}
          meta={entry.signal.source === "history" ? <span className="overview-edit__chip" data-chip="history" title="Saved activity from before this session">HISTORY</span>
            : <span className="overview-edit__chip" title="Observed change without a settled preview">CHANGED</span>} />
          : <p className="stream-notice" data-tone="warning" title={entry.detail}>{entry.message}</p>}
    </div>
  );
}
