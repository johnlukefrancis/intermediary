// Path: app/src/components/stream/stream_panel.tsx
// Description: The Stream panel shell: mode rocker with the LIVE slot, support states, and the live scroller

import type React from "react";
import { useCallback, useState } from "react";
import { AutoFilesHeader } from "../auto_files_header.js";
import { ContextMenu } from "../context_menu.js";
import { buildSingleFileContextMenuItems } from "../file_context_menu_items.js";
import { StreamHeaderLive, type StreamLiveState } from "./stream_header_live.js";
import { StreamScroller } from "./stream_scroller.js";
import type { RepoStream } from "../../hooks/stream/use_repo_stream.js";
import { useStreamImages } from "../../hooks/stream/use_stream_images.js";
import { useConfig } from "../../hooks/use_config.js";
import { useFileActions } from "../../hooks/use_file_actions.js";
import type { FileTypeFilter } from "../../lib/files/file_feed.js";
import type { FilesMode } from "../../lib/files/files_mode.js";
import { STREAM_MIN_AGENT_VERSION } from "../../lib/stream/stream_agent_support.js";
import { snapshotHasEntries } from "../../lib/stream/stream_store_support.js";

interface StreamPanelProps {
  repoId: string;
  repoLabel: string;
  filter: FileTypeFilter;
  mode: FilesMode;
  handset: boolean;
  headerPrefix?: React.ReactNode;
  stream: RepoStream;
  onFilterChange: (filter: FileTypeFilter) => void;
  onModeChange: (mode: FilesMode) => void;
}

export function StreamPanel({
  repoId,
  repoLabel,
  filter,
  mode,
  handset,
  headerPrefix,
  stream,
  onFilterChange,
  onModeChange,
}: StreamPanelProps): React.JSX.Element {
  const { config } = useConfig();
  const fileActions = useFileActions();
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; path: string } | null>(null);
  const repoRoot = config.repos.find((repo) => repo.repoId === repoId)?.root;

  const handleContextMenu = useCallback((event: React.MouseEvent, path: string) => {
    setContextMenu({ x: event.clientX, y: event.clientY, path });
  }, []);
  const closeContextMenu = useCallback(() => { setContextMenu(null); }, []);

  const { snapshot } = stream;
  // One tile owner per panel: it fetches only while this snapshot says the stream is visible
  const tiles = useStreamImages(repoId, snapshot);
  // Notices and the settling line render on the scroller even before the first card lands
  const ringEmpty = !snapshotHasEntries(snapshot);
  const liveState: StreamLiveState = snapshot.offline
    ? "offline"
    : snapshot.support === "update-required"
      ? "update"
      : snapshot.held
        ? "held"
        : "live";

  return (
    <section className="panel stream-panel" data-panel="stream">
      <header className="panel-header auto-files-panel-header">
        <AutoFilesHeader
          filter={filter}
          mode={mode}
          prefix={headerPrefix}
          liveSlot={<StreamHeaderLive state={liveState} />}
          onFilterChange={onFilterChange}
          onModeChange={onModeChange}
        />
      </header>
      <div className="panel-content stream-content">
        {snapshot.offline && ringEmpty ? (
          <p className="empty-state empty-state--waiting">Waiting for agent</p>
        ) : snapshot.support === "update-required" ? (
          <p className="empty-state empty-state--waiting">
            {`Agent update required · ${STREAM_MIN_AGENT_VERSION}+`}
          </p>
        ) : snapshot.held && ringEmpty ? (
          <p className="empty-state empty-state--waiting">WSL backend offline — stream held</p>
        ) : ringEmpty ? (
          <p className="empty-state empty-state--waiting">
            {`Watching ${repoLabel} — waiting for edits`}
          </p>
        ) : (
          <StreamScroller
            snapshot={snapshot}
            tiles={tiles}
            filter={filter}
            handset={handset}
            onExpand={stream.expand}
            onOpen={stream.openCard}
            onDrag={stream.dragCard}
            onContextMenu={handleContextMenu}
          />
        )}
        {contextMenu && repoRoot && (
          <ContextMenu
            x={contextMenu.x}
            y={contextMenu.y}
            items={buildSingleFileContextMenuItems({
              repoRoot,
              path: contextMenu.path,
              fileActions,
              logScope: "StreamPanel",
            })}
            onClose={closeContextMenu}
          />
        )}
      </div>
    </section>
  );
}
