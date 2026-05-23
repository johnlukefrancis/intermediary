// Path: app/src/components/bundles/bundle_explorer_directory.tsx
// Description: Recursive directory node for the lazy bundle file explorer

import type React from "react";
import { useCallback } from "react";
import type { BundleSelection } from "../../shared/protocol.js";
import { IndeterminateCheckbox } from "./indeterminate_checkbox.js";
import { BundleExplorerFileRow } from "./bundle_explorer_file_row.js";
import {
  baseName,
  directoryHasExclusions,
  isDirectoryEnabled,
  isDirectoryIncluded,
  isFileEnabled,
  isFileIncluded,
} from "./bundle_explorer_selection.js";

export interface DirectoryListingState {
  status: "idle" | "loading" | "ready" | "error";
  dirs: string[];
  files: string[];
  error?: string;
}

interface BundleExplorerDirectoryProps {
  path: string;
  depth: number;
  selection: BundleSelection;
  expandedDirs: ReadonlySet<string>;
  listings: ReadonlyMap<string, DirectoryListingState>;
  onToggleExpanded: (path: string) => void;
  onToggleDirectory: (path: string) => void;
  onToggleFile: (path: string) => void;
  onOpenFile: (path: string) => void;
  onFileContextMenu: (event: React.MouseEvent, path: string) => void;
}

function checkboxId(path: string): string {
  return `bundle-dir-${path.replace(/[^a-zA-Z0-9]/g, "-")}`;
}

export function BundleExplorerDirectory({
  path,
  depth,
  selection,
  expandedDirs,
  listings,
  onToggleExpanded,
  onToggleDirectory,
  onToggleFile,
  onOpenFile,
  onFileContextMenu,
}: BundleExplorerDirectoryProps): React.JSX.Element {
  const isExpanded = expandedDirs.has(path);
  const listing = listings.get(path) ?? { status: "idle", dirs: [], files: [] };
  const isEnabled = isDirectoryEnabled(path, selection);
  const isIncluded = isDirectoryIncluded(path, selection);
  const isIndeterminate = isIncluded && directoryHasExclusions(path, selection);
  const id = checkboxId(path);

  const handleExpand = useCallback(() => {
    onToggleExpanded(path);
  }, [onToggleExpanded, path]);

  return (
    <div className="bundle-explorer-dir">
      <div
        className={`bundle-explorer-dir-row bundle-explorer-row--depth-${Math.min(depth, 4)}`}
        data-disabled={!isEnabled || undefined}
      >
        <button
          className="dir-expand-btn"
          type="button"
          aria-label={isExpanded ? "Collapse" : "Expand"}
          aria-expanded={isExpanded}
          onClick={handleExpand}
        >
          {isExpanded ? "▼" : "▶"}
        </button>
        <IndeterminateCheckbox
          id={id}
          checked={isIncluded}
          indeterminate={isIndeterminate}
          disabled={!isEnabled}
          onChange={() => { onToggleDirectory(path); }}
        />
        <label className="bundle-explorer-dir-name" htmlFor={id} title={path}>
          {baseName(path)}
        </label>
      </div>

      {isExpanded && (
        <div className="bundle-explorer-children">
          {listing.status === "loading" && (
            <span className="bundle-explorer-status">Loading</span>
          )}
          {listing.status === "error" && (
            <span className="bundle-explorer-status bundle-explorer-status--error">
              {listing.error ?? "Unable to load directory"}
            </span>
          )}
          {listing.status === "ready" && listing.files.map((filePath) => (
            <BundleExplorerFileRow
              key={filePath}
              path={filePath}
              depth={depth + 1}
              enabled={isFileEnabled(filePath, selection)}
              included={isFileIncluded(filePath, selection)}
              onToggle={onToggleFile}
              onOpen={onOpenFile}
              onContextMenu={onFileContextMenu}
            />
          ))}
          {listing.status === "ready" && listing.dirs.map((dirPath) => (
            <BundleExplorerDirectory
              key={dirPath}
              path={dirPath}
              depth={depth + 1}
              selection={selection}
              expandedDirs={expandedDirs}
              listings={listings}
              onToggleExpanded={onToggleExpanded}
              onToggleDirectory={onToggleDirectory}
              onToggleFile={onToggleFile}
              onOpenFile={onOpenFile}
              onFileContextMenu={onFileContextMenu}
            />
          ))}
        </div>
      )}
    </div>
  );
}
