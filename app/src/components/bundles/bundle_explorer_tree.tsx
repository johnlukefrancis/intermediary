// Path: app/src/components/bundles/bundle_explorer_tree.tsx
// Description: Top-level directory/file list for the bundle explorer, carrying drop-target attributes and tree keyboard focus

import type React from "react";
import type { BundleSelection } from "../../shared/protocol.js";
import { BundleExplorerDirectory } from "./bundle_explorer_directory.js";
import { BundleExplorerFileRow } from "./bundle_explorer_file_row.js";
import { isFileEnabled, isFileIncluded } from "../../lib/bundles/bundle_selection_visibility.js";
import type { DirectoryListingState } from "../../hooks/bundles/use_directory_listings.js";
import { useRootDropTarget } from "./tree_interaction_context.js";

interface BundleExplorerTreeProps {
  topLevelDirs: string[];
  topLevelFiles: string[];
  selection: BundleSelection;
  expandedDirs: ReadonlySet<string>;
  listings: ReadonlyMap<string, DirectoryListingState>;
  renameInFlight: boolean;
  isDragActive: boolean;
  listRef: React.RefObject<HTMLDivElement>;
  onToggleExpanded: (path: string) => void;
  onToggleDirectory: (path: string) => void;
  onToggleFile: (path: string) => void;
  onOpenFile: (path: string) => void;
  onKeyDown: (event: React.KeyboardEvent) => void;
  onBlankContextMenu: (event: React.MouseEvent) => void;
}

export function BundleExplorerTree({
  topLevelDirs,
  topLevelFiles,
  selection,
  expandedDirs,
  listings,
  renameInFlight,
  isDragActive,
  listRef,
  onToggleExpanded,
  onToggleDirectory,
  onToggleFile,
  onOpenFile,
  onKeyDown,
  onBlankContextMenu,
}: BundleExplorerTreeProps): React.JSX.Element {
  const isRootDropTarget = useRootDropTarget();

  return (
    <div
      className="bundle-explorer-list"
      ref={listRef}
      tabIndex={0}
      data-drop-dir=""
      data-drop-active={isDragActive || undefined}
      data-drop-target={isRootDropTarget || undefined}
      onKeyDown={onKeyDown}
      onContextMenu={(event) => {
        if (event.target !== event.currentTarget) return;
        event.preventDefault();
        onBlankContextMenu(event);
      }}
    >
      {topLevelDirs.map((dirPath) => (
        <BundleExplorerDirectory
          key={dirPath}
          path={dirPath}
          depth={0}
          selection={selection}
          expandedDirs={expandedDirs}
          listings={listings}
          renameInFlight={renameInFlight}
          onToggleExpanded={onToggleExpanded}
          onToggleDirectory={onToggleDirectory}
          onToggleFile={onToggleFile}
          onOpenFile={onOpenFile}
        />
      ))}
      {topLevelFiles.map((filePath) => (
        <BundleExplorerFileRow
          key={filePath}
          path={filePath}
          depth={0}
          enabled={isFileEnabled(filePath, selection)}
          included={isFileIncluded(filePath, selection)}
          renameInFlight={renameInFlight}
          onToggle={onToggleFile}
          onOpen={onOpenFile}
        />
      ))}
      {topLevelFiles.length === 0 && topLevelDirs.length === 0 && (
        <span className="no-dirs">No files found</span>
      )}
    </div>
  );
}
