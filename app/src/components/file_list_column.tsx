// Path: app/src/components/file_list_column.tsx
// Description: Column wrapper that renders a list of FileRow components with context menu

import type React from "react";
import { useCallback, useState } from "react";
import { FileRow } from "./file_row.js";
import { ContextMenu } from "./context_menu.js";
import type { ContextMenuItem } from "./context_menu.js";
import type { FileEntry, StagedInfo } from "../shared/protocol.js";
import { useStarredFiles } from "../hooks/use_starred_files.js";
import { useConfig } from "../hooks/use_config.js";
import { useFileActions } from "../hooks/use_file_actions.js";

interface FileListColumnProps {
  files: FileEntry[];
  stagedByPath: Map<string, StagedInfo>;
  repoId: string;
  kind: "docs" | "code";
  emptyMessage?: string;
  onDragStart: (
    repoId: string,
    relativePath: string,
    stagedInfo: StagedInfo | undefined
  ) => void | Promise<void>;
}

interface ContextMenuState {
  x: number;
  y: number;
  file: FileEntry;
}

export function FileListColumn({
  files,
  stagedByPath,
  repoId,
  kind,
  emptyMessage = "No files",
  onDragStart,
}: FileListColumnProps): React.JSX.Element {
  const { isStarred, toggle } = useStarredFiles(repoId);
  const { config } = useConfig();
  const fileActions = useFileActions();
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);

  const repoRoot = config.repos.find((r) => r.repoId === repoId)?.root;

  const handleContextMenu = useCallback(
    (e: React.MouseEvent, file: FileEntry) => {
      setContextMenu({ x: e.clientX, y: e.clientY, file });
    },
    []
  );

  const closeContextMenu = useCallback(() => { setContextMenu(null); }, []);

  if (files.length === 0) {
    const isWaiting = emptyMessage.toLowerCase().includes("waiting");
    const className = isWaiting ? "empty-state empty-state--waiting" : "empty-state";
    return <p className={className}>{emptyMessage}</p>;
  }

  const contextMenuItems: ContextMenuItem[] = [];
  if (contextMenu && repoRoot) {
    const { file } = contextMenu;
    contextMenuItems.push(
      {
        label: "Open Containing Folder",
        onClick: () => { void fileActions.revealInFileManager(repoRoot, file.path); },
      },
      {
        label: "Open File",
        onClick: () => { void fileActions.openFile(repoRoot, file.path); },
      },
      {
        label: isStarred(kind, file.path) ? "Unfavourite" : "Favourite",
        onClick: () => { toggle(kind, file.path); },
      }
    );
  }

  return (
    <div className="file-list">
      {files.map((file) => (
        <FileRow
          key={file.path}
          file={file}
          repoId={repoId}
          stagedInfo={stagedByPath.get(file.path)}
          isStarred={isStarred(kind, file.path)}
          onDragStart={onDragStart}
          onToggleStar={() => { toggle(kind, file.path); }}
          onContextMenu={handleContextMenu}
        />
      ))}
      {contextMenu && repoRoot && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          items={contextMenuItems}
          onClose={closeContextMenu}
        />
      )}
    </div>
  );
}
