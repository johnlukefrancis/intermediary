// Path: app/src/components/bundles/bundle_file_explorer.tsx
// Description: Lazy file explorer for bundle directory and file inclusion

import type React from "react";
import { useCallback, useState } from "react";
import type { BundleSelection } from "../../shared/protocol.js";
import { useConfig } from "../../hooks/use_config.js";
import { useDirectoryListings } from "../../hooks/bundles/use_directory_listings.js";
import { BundleExplorerDirectory } from "./bundle_explorer_directory.js";
import { BundleFileContextMenu } from "./bundle_file_context_menu.js";
import { BundleExplorerFileRow } from "./bundle_explorer_file_row.js";
import {
  isFileEnabled,
  isFileIncluded,
  isSelfOrDescendant,
  sortedWith,
  withoutPath,
} from "../../lib/bundles/bundle_selection_visibility.js";

interface BundleFileExplorerProps {
  repoId: string;
  selection: BundleSelection;
  topLevelDirs: string[];
  topLevelFiles: string[];
  onSelectionChange: (selection: BundleSelection) => void;
  onOpenFile: (path: string) => void;
}

interface ContextMenuState {
  x: number;
  y: number;
  path: string;
}

export function BundleFileExplorer({
  repoId,
  selection,
  topLevelDirs,
  topLevelFiles,
  onSelectionChange,
  onOpenFile,
}: BundleFileExplorerProps): React.JSX.Element {
  const { config } = useConfig();
  const { expandedDirs, listings, toggleExpanded } = useDirectoryListings({
    repoId,
    topLevelDirs,
    topLevelFiles,
  });
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  const repoRoot = config.repos.find((repo) => repo.repoId === repoId)?.root;

  const allSelected =
    topLevelDirs.length > 0 && selection.topLevelDirs.length === topLevelDirs.length;
  const noneSelected = selection.topLevelDirs.length === 0;

  const handleIncludeRootChange = useCallback(
    (event: React.ChangeEvent<HTMLInputElement>) => {
      onSelectionChange({ ...selection, includeRoot: event.target.checked });
    },
    [onSelectionChange, selection]
  );

  const handleSelectAll = useCallback(() => {
    onSelectionChange({ ...selection, topLevelDirs: [...topLevelDirs].sort() });
  }, [onSelectionChange, selection, topLevelDirs]);

  const handleSelectNone = useCallback(() => {
    onSelectionChange({
      ...selection,
      topLevelDirs: [],
      includedSubdirs: [],
      excludedSubdirs: [],
      excludedFiles: selection.excludedFiles.filter((value) => !value.includes("/")),
    });
  }, [onSelectionChange, selection]);

  const handleToggleDirectory = useCallback(
    (path: string) => {
      if (!path.includes("/")) {
        const selected = new Set(selection.topLevelDirs);
        if (selected.has(path)) {
          selected.delete(path);
          onSelectionChange({
            ...selection,
            topLevelDirs: [...selected].sort(),
            includedSubdirs: selection.includedSubdirs.filter(
              (value) => !isSelfOrDescendant(value, path)
            ),
            excludedSubdirs: selection.excludedSubdirs.filter((value) => !isSelfOrDescendant(value, path)),
            excludedFiles: selection.excludedFiles.filter((value) => !isSelfOrDescendant(value, path)),
          });
        } else {
          selected.add(path);
          onSelectionChange({ ...selection, topLevelDirs: [...selected].sort() });
        }
        return;
      }

      const isExcluded = selection.excludedSubdirs.includes(path);
      const excludedSubdirs = isExcluded
        ? withoutPath(path, selection.excludedSubdirs)
        : sortedWith(path, selection.excludedSubdirs);
      const includedSubdirs = isExcluded
        ? sortedWith(path, selection.includedSubdirs)
        : selection.includedSubdirs.filter(
            (value) => !isSelfOrDescendant(value, path)
          );
      onSelectionChange({ ...selection, includedSubdirs, excludedSubdirs });
    },
    [onSelectionChange, selection]
  );

  const handleToggleFile = useCallback(
    (path: string) => {
      const excludedFiles = selection.excludedFiles.includes(path)
        ? withoutPath(path, selection.excludedFiles)
        : sortedWith(path, selection.excludedFiles);
      onSelectionChange({ ...selection, excludedFiles });
    },
    [onSelectionChange, selection]
  );

  const handleFileContextMenu = useCallback((event: React.MouseEvent, path: string) => {
    setContextMenu({ x: event.clientX, y: event.clientY, path });
  }, []);

  const closeContextMenu = useCallback(() => { setContextMenu(null); }, []);

  return (
    <div className="bundle-file-explorer">
      <div className="selection-header">
        <div className="include-root-toggle">
          <label className="vintage-toggle">
            <input
              id="include-root-checkbox"
              type="checkbox"
              checked={selection.includeRoot}
              onChange={handleIncludeRootChange}
            />
            <span className="vintage-toggle-track" />
          </label>
          <label className="toggle-label" htmlFor="include-root-checkbox">
            Include root files
          </label>
        </div>
      </div>

      <div className="dir-selection-header">
        <span>Files</span>
        <div className="dir-selection-actions">
          <button
            type="button"
            className="dir-action-btn"
            onClick={handleSelectAll}
            disabled={topLevelDirs.length === 0 || allSelected}
          >
            All
          </button>
          <button
            type="button"
            className="dir-action-btn"
            onClick={handleSelectNone}
            disabled={topLevelDirs.length === 0 || noneSelected}
          >
            None
          </button>
        </div>
      </div>

      <div className="bundle-explorer-list">
        {topLevelDirs.map((dirPath) => (
          <BundleExplorerDirectory
            key={dirPath}
            path={dirPath}
            depth={0}
            selection={selection}
            expandedDirs={expandedDirs}
            listings={listings}
            onToggleExpanded={toggleExpanded}
            onToggleDirectory={handleToggleDirectory}
            onToggleFile={handleToggleFile}
            onOpenFile={onOpenFile}
            onFileContextMenu={handleFileContextMenu}
          />
        ))}
        {topLevelFiles.map((filePath) => (
          <BundleExplorerFileRow
            key={filePath}
            path={filePath}
            depth={0}
            enabled={isFileEnabled(filePath, selection)}
            included={isFileIncluded(filePath, selection)}
            onToggle={handleToggleFile}
            onOpen={onOpenFile}
            onContextMenu={handleFileContextMenu}
          />
        ))}
        {topLevelFiles.length === 0 && topLevelDirs.length === 0 && (
          <span className="no-dirs">No files found</span>
        )}
      </div>

      {contextMenu && repoRoot && (
        <BundleFileContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          path={contextMenu.path}
          repoRoot={repoRoot}
          onClose={closeContextMenu}
        />
      )}
    </div>
  );
}
