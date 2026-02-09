// Path: app/src/components/file_list_column.tsx
// Description: Column wrapper that renders a list of FileRow components with context menu

import type React from "react";
import { useCallback, useState } from "react";
import { FileRow } from "./file_row.js";
import { ContextMenu } from "./context_menu.js";
import type { ContextMenuItem } from "./context_menu.js";
import type { FileEntry } from "../shared/protocol.js";
import { useStarredFiles } from "../hooks/use_starred_files.js";
import { useConfig } from "../hooks/use_config.js";
import { useFileActions } from "../hooks/use_file_actions.js";

interface FileListColumnProps {
  files: FileEntry[];
  repoId: string;
  kind: "docs" | "code";
  emptyMessage?: string;
  selectedPaths: ReadonlySet<string>;
  onSelect: (path: string, event: React.MouseEvent) => void;
  onDragStart: (path: string, event: React.MouseEvent) => void | Promise<void>;
}

interface ContextMenuState {
  x: number;
  y: number;
  file: FileEntry;
}

export function FileListColumn({
  files,
  repoId,
  kind,
  emptyMessage = "No files",
  selectedPaths,
  onSelect,
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
    const isMulti = selectedPaths.has(file.path) && selectedPaths.size > 1;

    if (isMulti) {
      const selected = files
        .map((entry) => entry.path)
        .filter((path) => selectedPaths.has(path));
      const allSelectedStarred = selected.every((path) => isStarred(kind, path));
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
        },
        {
          label: allSelectedStarred ? "Unfavourite All" : "Favourite All",
          onClick: () => {
            for (const path of selected) {
              const selectedPathIsStarred = isStarred(kind, path);
              if (allSelectedStarred ? selectedPathIsStarred : !selectedPathIsStarred) {
                toggle(kind, path);
              }
            }
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
        },
        {
          label: isStarred(kind, file.path) ? "Unfavourite" : "Favourite",
          onClick: () => { toggle(kind, file.path); },
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
          isStarred={isStarred(kind, file.path)}
          isSelected={selectedPaths.has(file.path)}
          onDragStart={onDragStart}
          onSelect={onSelect}
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
