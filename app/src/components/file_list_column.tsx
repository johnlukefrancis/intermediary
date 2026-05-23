// Path: app/src/components/file_list_column.tsx
// Description: Column wrapper that renders file feed rows with context menu actions

import type React from "react";
import { useCallback, useState } from "react";
import { FileRow } from "./file_row.js";
import { ContextMenu } from "./context_menu.js";
import type { ContextMenuItem } from "./context_menu.js";
import type { FeedFileEntry } from "../lib/files/file_feed.js";
import { useConfig } from "../hooks/use_config.js";
import { useFileActions } from "../hooks/use_file_actions.js";

interface FileListColumnProps {
  files: FeedFileEntry[];
  repoId: string;
  emptyMessage?: string;
  selectedPaths: ReadonlySet<string>;
  onSelect: (
    path: string,
    event: Pick<React.MouseEvent, "ctrlKey" | "metaKey" | "shiftKey">
  ) => void;
  onDragStart: (path: string) => void | Promise<void>;
  onOpen: (path: string) => void;
}

interface ContextMenuState {
  x: number;
  y: number;
  file: FeedFileEntry;
}

export function FileListColumn({
  files,
  repoId,
  emptyMessage = "No files",
  selectedPaths,
  onSelect,
  onDragStart,
  onOpen,
}: FileListColumnProps): React.JSX.Element {
  const { config } = useConfig();
  const fileActions = useFileActions();
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);

  const repoRoot = config.repos.find((r) => r.repoId === repoId)?.root;

  const handleContextMenu = useCallback(
    (e: React.MouseEvent, file: FeedFileEntry) => {
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
    const isMulti = selectedPaths.has(file.path) && selectedPaths.size > 1;

    if (isMulti) {
      const selected = files
        .map((entry) => entry.path)
        .filter((path) => selectedPaths.has(path));
      contextMenuItems.push(
        {
          label: `${selected.length} files selected`,
          onClick: () => {},
          disabled: true,
        },
        {
          label: "Open Containing Folder",
          onClick: () => {
            const firstFilePerDir = new Map<string, string>();
            for (const path of selected) {
              const idx = path.lastIndexOf("/");
              const dir = idx === -1 ? "" : path.slice(0, idx);
              if (!firstFilePerDir.has(dir)) {
                firstFilePerDir.set(dir, path);
              }
            }

            for (const representativePath of firstFilePerDir.values()) {
              void fileActions.revealInFileManager(repoRoot, representativePath);
            }
          },
        },
        {
          label: "Open All Files",
          onClick: () => {
            void fileActions.openFiles(repoRoot, selected);
          },
        },
        {
          label: "Copy Relative Paths",
          onClick: () => {
            void navigator.clipboard.writeText(selected.join("\n")).catch((err: unknown) => {
              console.error("[ContextMenu] copy_relative_paths failed:", err);
            });
          },
        }
      );
    } else {
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
          label: "Copy Relative Path",
          onClick: () => {
            void navigator.clipboard.writeText(file.path).catch((err: unknown) => {
              console.error("[ContextMenu] copy_relative_path failed:", err);
            });
          },
        }
      );
    }
  }

  return (
    <div className="file-list">
      {files.map((file) => (
        <FileRow
          key={file.path}
          file={file}
          isSelected={selectedPaths.has(file.path)}
          activityBadge={file.activityBadge}
          onDragStart={onDragStart}
          onSelect={onSelect}
          onOpen={onOpen}
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
