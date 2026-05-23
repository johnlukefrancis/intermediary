// Path: app/src/components/bundles/bundle_file_explorer.tsx
// Description: Lazy file explorer for bundle directory and file inclusion

import type React from "react";
import { useCallback, useEffect, useRef, useState } from "react";
import type { BundleSelection } from "../../shared/protocol.js";
import { sendListRepoDirectory } from "../../lib/agent/messages.js";
import { useAgent } from "../../hooks/use_agent.js";
import { useConfig } from "../../hooks/use_config.js";
import { BundleExplorerDirectory, type DirectoryListingState } from "./bundle_explorer_directory.js";
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
  const { client, helloState } = useAgent();
  const { config } = useConfig();
  const [expandedDirs, setExpandedDirs] = useState<Set<string>>(() => new Set());
  const [listings, setListings] = useState<Map<string, DirectoryListingState>>(() => new Map());
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  const listingScopeRef = useRef({ repoId, generation: 0 });
  const repoRoot = config.repos.find((repo) => repo.repoId === repoId)?.root;

  const allSelected =
    topLevelDirs.length > 0 && selection.topLevelDirs.length === topLevelDirs.length;
  const noneSelected = selection.topLevelDirs.length === 0;

  useEffect(() => {
    listingScopeRef.current = {
      repoId,
      generation: listingScopeRef.current.generation + 1,
    };
    setExpandedDirs(new Set());
    setListings(new Map());
  }, [repoId, topLevelDirs, topLevelFiles]);

  const loadDirectory = useCallback(
    (path: string) => {
      if (!client || helloState.status !== "ok") {
        setListings((prev) => new Map(prev).set(path, {
          status: "error",
          dirs: [],
          files: [],
          error: "Agent session initializing",
        }));
        return;
      }

      const current = listings.get(path);
      if (current?.status === "loading" || current?.status === "ready") return;

      setListings((prev) => new Map(prev).set(path, { status: "loading", dirs: [], files: [] }));
      const requestScope = listingScopeRef.current;
      const requestRepoId = repoId;
      const requestPath = path;
      void sendListRepoDirectory(client, requestRepoId, requestPath)
        .then((result) => {
          const activeScope = listingScopeRef.current;
          if (
            activeScope.repoId !== requestRepoId ||
            activeScope.generation !== requestScope.generation ||
            result.repoId !== requestRepoId ||
            result.path !== requestPath
          ) {
            return;
          }
          setListings((prev) => new Map(prev).set(requestPath, {
            status: "ready",
            dirs: result.dirs,
            files: result.files,
          }));
        })
        .catch((error: unknown) => {
          const activeScope = listingScopeRef.current;
          if (
            activeScope.repoId !== requestRepoId ||
            activeScope.generation !== requestScope.generation
          ) {
            return;
          }
          const message = error instanceof Error ? error.message : "Unable to load directory";
          setListings((prev) => new Map(prev).set(requestPath, {
            status: "error",
            dirs: [],
            files: [],
            error: message,
          }));
        });
    },
    [client, helloState.status, listings, repoId]
  );

  const handleToggleExpanded = useCallback((path: string) => {
    setExpandedDirs((prev) => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
        loadDirectory(path);
      }
      return next;
    });
  }, [loadDirectory]);

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
            excludedSubdirs: selection.excludedSubdirs.filter((value) => !isSelfOrDescendant(value, path)),
            excludedFiles: selection.excludedFiles.filter((value) => !isSelfOrDescendant(value, path)),
          });
        } else {
          selected.add(path);
          onSelectionChange({ ...selection, topLevelDirs: [...selected].sort() });
        }
        return;
      }

      const excludedSubdirs = selection.excludedSubdirs.includes(path)
        ? withoutPath(path, selection.excludedSubdirs)
        : sortedWith(path, selection.excludedSubdirs);
      onSelectionChange({ ...selection, excludedSubdirs });
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
            onToggleExpanded={handleToggleExpanded}
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
