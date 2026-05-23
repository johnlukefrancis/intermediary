// Path: app/src/components/auto_files_panel.tsx
// Description: Unified Auto files panel with ranked telemetry table

import type React from "react";
import { useCallback, useState } from "react";
import { ContextMenu, type ContextMenuItem } from "./context_menu.js";
import { buildSingleFileContextMenuItems } from "./file_context_menu_items.js";
import { AutoFilesHeader } from "./auto_files_header.js";
import { AutoFilesRow } from "./auto_files_row.js";
import { useConfig } from "../hooks/use_config.js";
import { useFileActions } from "../hooks/use_file_actions.js";
import type {
  FeedFileEntry,
  FileSortMode,
  FileTypeFilter,
} from "../lib/files/file_feed.js";
import type { RepoRoot } from "../shared/config.js";

interface AutoFilesPanelProps {
  files: FeedFileEntry[];
  repoId: string;
  emptyMessage?: string;
  filter: FileTypeFilter;
  sortMode: FileSortMode;
  selectedPaths: ReadonlySet<string>;
  headerPrefix?: React.ReactNode;
  onFilterChange: (filter: FileTypeFilter) => void;
  onSortModeChange: (mode: FileSortMode) => void;
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

export function AutoFilesPanel({
  files,
  repoId,
  emptyMessage = "No files",
  filter,
  sortMode,
  selectedPaths,
  headerPrefix,
  onFilterChange,
  onSortModeChange,
  onSelect,
  onDragStart,
  onOpen,
}: AutoFilesPanelProps): React.JSX.Element {
  const { config } = useConfig();
  const fileActions = useFileActions();
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  const repoRoot = config.repos.find((repo) => repo.repoId === repoId)?.root;

  const handleContextMenu = useCallback((e: React.MouseEvent, file: FeedFileEntry) => {
    setContextMenu({ x: e.clientX, y: e.clientY, file });
  }, []);

  const closeContextMenu = useCallback(() => { setContextMenu(null); }, []);
  const contextMenuItems = buildContextMenuItems({
    contextMenu,
    files,
    repoRoot,
    selectedPaths,
    fileActions,
  });

  const isWaiting = emptyMessage.toLowerCase().includes("waiting");

  return (
    <section className="panel auto-files-panel" data-panel="auto-files">
      <header className="panel-header auto-files-panel-header">
        <AutoFilesHeader
          filter={filter}
          sortMode={sortMode}
          prefix={headerPrefix}
          onFilterChange={onFilterChange}
          onSortModeChange={onSortModeChange}
        />
      </header>
      <div className="panel-content auto-files-content">
        {files.length === 0 ? (
          <p className={isWaiting ? "empty-state empty-state--waiting" : "empty-state"}>
            {emptyMessage}
          </p>
        ) : (
          <div className="auto-files-table" role="table" aria-label="Files">
            <div className="auto-files-table-head" role="row">
              <span>#</span>
              <span>File</span>
              <span data-emphasis={sortMode === "latest" ? true : undefined}>Last Active</span>
              <span aria-label="Update count">Count</span>
              <span
                data-emphasis={
                  sortMode === "active" || sortMode === "auto" ? true : undefined
                }
              >
                Activity
              </span>
            </div>
            <div className="auto-files-table-body">
              {files.map((file, index) => (
                <AutoFilesRow
                  key={file.path}
                  file={file}
                  rank={index + 1}
                  isSelected={selectedPaths.has(file.path)}
                  sortMode={sortMode}
                  onDragStart={onDragStart}
                  onSelect={onSelect}
                  onOpen={onOpen}
                  onContextMenu={handleContextMenu}
                />
              ))}
            </div>
          </div>
        )}
        {contextMenu && repoRoot && (
          <ContextMenu
            x={contextMenu.x}
            y={contextMenu.y}
            items={contextMenuItems}
            onClose={closeContextMenu}
          />
        )}
      </div>
    </section>
  );
}

function buildContextMenuItems(input: {
  contextMenu: ContextMenuState | null;
  files: FeedFileEntry[];
  repoRoot: RepoRoot | undefined;
  selectedPaths: ReadonlySet<string>;
  fileActions: ReturnType<typeof useFileActions>;
}): ContextMenuItem[] {
  const { contextMenu, files, repoRoot, selectedPaths, fileActions } = input;
  if (!contextMenu || !repoRoot) return [];

  const { file } = contextMenu;
  const isMulti = selectedPaths.has(file.path) && selectedPaths.size > 1;
  if (!isMulti) {
    return buildSingleFileContextMenuItems({
      repoRoot,
      path: file.path,
      fileActions,
      logScope: "AutoFilesPanel",
    });
  }

  const selected = files.map((entry) => entry.path).filter((path) => selectedPaths.has(path));
  return [
    { label: `${selected.length} files selected`, onClick: () => {}, disabled: true },
    {
      label: "Open Containing Folder",
      onClick: () => {
        for (const path of firstFilePerDirectory(selected)) {
          void fileActions.revealInFileManager(repoRoot, path);
        }
      },
    },
    { label: "Open All Files", onClick: () => { void fileActions.openFiles(repoRoot, selected); } },
    {
      label: "Copy Relative Paths",
      onClick: () => {
        void navigator.clipboard.writeText(selected.join("\n")).catch((err: unknown) => {
          console.error("[ContextMenu] copy_relative_paths failed:", err);
        });
      },
    },
  ];
}

function firstFilePerDirectory(paths: string[]): string[] {
  const firstByDir = new Map<string, string>();
  for (const path of paths) {
    const index = path.lastIndexOf("/");
    const directory = index === -1 ? "" : path.slice(0, index);
    if (!firstByDir.has(directory)) firstByDir.set(directory, path);
  }
  return [...firstByDir.values()];
}
