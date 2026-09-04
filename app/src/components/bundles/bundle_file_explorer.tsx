// Path: app/src/components/bundles/bundle_file_explorer.tsx
// Description: Lazy file explorer for bundle directory/file inclusion, selection, clipboard, drag-move, and rename

import { topmostPaths } from "../../lib/bundles/bundle_selection_visibility.js";
import type React from "react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { BundleSelection } from "../../shared/protocol.js";
import { useConfig } from "../../hooks/use_config.js";
import { useDirectoryListings } from "../../hooks/bundles/use_directory_listings.js";
import { useTreeDropImport } from "../../hooks/bundles/use_tree_drop_import.js";
import { useImportRequest } from "../../hooks/bundles/use_import_request.js";
import { useTreeSelection } from "../../hooks/bundles/use_tree_selection.js";
import { useTreeClipboard } from "../../hooks/bundles/use_tree_clipboard.js";
import { useTreeKeyboard } from "../../hooks/bundles/use_tree_keyboard.js";
import { useTreeRowDrag } from "../../hooks/bundles/use_tree_row_drag.js";
import {
  useEntryActionRequest,
  type EntryActionKind,
} from "../../hooks/bundles/use_entry_action_request.js";
import { useBundleInclusion } from "../../hooks/bundles/use_bundle_inclusion.js";
import { flattenVisibleTree } from "../../lib/bundles/flatten_visible_tree.js";
import { parentPath } from "../../lib/bundles/bundle_selection_visibility.js";
import { BundleExplorerTree } from "./bundle_explorer_tree.js";
import { BundleExplorerRowMenu } from "./bundle_explorer_row_menu.js";
import { BundleEntriesFeedback } from "./bundle_entries_feedback.js";
import { BundleDragGhost } from "./bundle_drag_ghost.js";
import { ConfirmModal } from "../confirm_modal.js";
import { ContextMenu } from "../context_menu.js";
import {
  TreeInteractionProvider,
  type TreeRowMenuRequest,
} from "./tree_interaction_context.js";

interface BundleFileExplorerProps {
  repoId: string;
  selection: BundleSelection;
  topLevelDirs: string[];
  topLevelFiles: string[];
  onSelectionChange: (selection: BundleSelection) => void;
  onOpenFile: (path: string) => void;
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
  const {
    expandedDirs, listings, toggleExpanded, expandDirectory, refreshDirectory, forgetSubtree,
  } = useDirectoryListings({ repoId, topLevelDirs, topLevelFiles });
  const repoRoot = config.repos.find((repo) => repo.repoId === repoId)?.root;
  const listRef = useRef<HTMLDivElement>(null);
  const inclusion = useBundleInclusion({ selection, topLevelDirs, onSelectionChange });

  const visibleRows = useMemo(
    () => flattenVisibleTree(topLevelDirs, topLevelFiles, expandedDirs, listings),
    [topLevelDirs, topLevelFiles, expandedDirs, listings]
  );
  const treeSelection = useTreeSelection({ repoId, visibleRows, listings });
  const clipboard = useTreeClipboard(repoId);
  const [renamingPath, setRenamingPath] = useState<string | null>(null);
  const [rowMenu, setRowMenu] = useState<TreeRowMenuRequest | null>(null);
  const [blankMenu, setBlankMenu] = useState<{ x: number; y: number } | null>(null);
  const [pendingDelete, setPendingDelete] = useState<string[] | null>(null);

  // Per-repo mutation-local UI state must not survive a repo switch (RepoTab renders unkeyed).
  useEffect(() => {
    setRenamingPath(null);
    setRowMenu(null);
    setBlankMenu(null);
    setPendingDelete(null);
  }, [repoId]);

  const handleApplied = useCallback(
    (kind: EntryActionKind, entries: string[], sourcePaths: string[]) => {
      if (kind !== "copy") {
        for (const path of sourcePaths) forgetSubtree(path);
      }
      for (const parent of new Set(sourcePaths.map(parentPath))) refreshDirectory(parent);
      if (kind !== "delete" && entries.length > 0) refreshDirectory(parentPath(entries[0] as string));
      if (kind === "rename") {
        treeSelection.replaceWith(entries);
        setRenamingPath(null);
      } else {
        treeSelection.clear();
      }
      if (kind === "move" && clipboard.clipboard?.mode === "cut") clipboard.clear();
    },
    [clipboard, forgetSubtree, refreshDirectory, treeSelection]
  );
  const actions = useEntryActionRequest({ repoId, onApplied: handleApplied });

  const handlePaste = useCallback(
    (directory: string) => {
      const clip = clipboard.clipboard;
      if (!clip) return;
      if (clip.mode === "cut") {
        actions.moveEntries(clip.paths, directory);
      } else {
        actions.copyEntries(clip.paths, directory);
      }
    },
    [actions, clipboard.clipboard]
  );

  const resolvePasteTarget = useCallback((): string => {
    const paths = [...treeSelection.selected];
    if (paths.length !== 1) return "";
    const row = visibleRows.find((candidate) => candidate.path === paths[0]);
    return row?.kind === "dir" ? (paths[0] as string) : parentPath(paths[0] as string);
  }, [treeSelection.selected, visibleRows]);

  const {
    importFiles, inFlight: importInFlight, pendingReplace: importReplace, confirmReplace: confirmImport,
    cancelReplace: cancelImport, error: importError, dismissError: dismissImportError,
  } = useImportRequest({ repoId });
  const { dropTargetDir: osDropTargetDir, isDragActive } = useTreeDropImport({
    listRef, expandedDirs, expandDirectory, onImport: importFiles, importInFlight,
  });
  const rowDrag = useTreeRowDrag({
    listRef,
    selected: treeSelection.selected,
    expandedDirs,
    expandDirectory,
    onDrop: (paths, directory) => { actions.moveEntries(paths, directory); },
  });
  const mergedDropTargetDir = osDropTargetDir ?? rowDrag.dropTargetDir;

  const keyboard = useTreeKeyboard({
    visibleRows,
    selected: treeSelection.selected,
    anchor: treeSelection.anchor,
    expandedDirs,
    renaming: renamingPath,
    selectOnly: treeSelection.selectOnly,
    rangeTo: treeSelection.rangeTo,
    clearSelection: treeSelection.clear,
    expandDirectory,
    toggleExpanded,
    onOpenFile,
    onDeleteRequest: setPendingDelete,
    onRenameStart: setRenamingPath,
    onRenameCancel: () => { setRenamingPath(null); },
    onCut: clipboard.cut,
    onCopy: clipboard.copy,
    onPaste: () => { handlePaste(resolvePasteTarget()); },
  });

  return (
    <div className="bundle-file-explorer">
      <div className="selection-header">
        <div className="include-root-toggle">
          <label className="vintage-toggle">
            <input
              id="include-root-checkbox"
              type="checkbox"
              checked={selection.includeRoot}
              onChange={inclusion.handleIncludeRootChange}
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
            onClick={inclusion.handleSelectAll}
            disabled={topLevelDirs.length === 0 || inclusion.allSelected}
          >
            All
          </button>
          <button
            type="button"
            className="dir-action-btn"
            onClick={inclusion.handleSelectNone}
            disabled={topLevelDirs.length === 0 || inclusion.noneSelected}
          >
            None
          </button>
        </div>
      </div>

      <TreeInteractionProvider
        selected={treeSelection.selected}
        cutPaths={new Set(clipboard.clipboard?.mode === "cut" ? clipboard.clipboard.paths : [])}
        dropTargetDir={mergedDropTargetDir}
        renaming={renamingPath}
        visibleRows={visibleRows}
        expandedDirs={expandedDirs}
        selectOnly={treeSelection.selectOnly}
        toggle={treeSelection.toggle}
        rangeTo={treeSelection.rangeTo}
        toggleExpanded={toggleExpanded}
        onStartDrag={rowDrag.startDrag}
        onOpenMenu={setRowMenu}
        onRenameCommit={(newName) => { if (renamingPath) actions.renameEntry(renamingPath, newName); }}
        onRenameCancel={() => { setRenamingPath(null); }}
      >
        <BundleExplorerTree
          topLevelDirs={topLevelDirs}
          topLevelFiles={topLevelFiles}
          selection={selection}
          expandedDirs={expandedDirs}
          listings={listings}
          renameInFlight={actions.inFlight}
          isDragActive={isDragActive}
          listRef={listRef}
          onToggleExpanded={toggleExpanded}
          onToggleDirectory={inclusion.handleToggleDirectory}
          onToggleFile={inclusion.handleToggleFile}
          onOpenFile={onOpenFile}
          onKeyDown={keyboard.onKeyDown}
          onBlankContextMenu={(event) => { setBlankMenu({ x: event.clientX, y: event.clientY }); }}
        />
      </TreeInteractionProvider>

      {rowMenu && repoRoot && (
        <BundleExplorerRowMenu
          x={rowMenu.x}
          y={rowMenu.y}
          path={rowMenu.path}
          kind={rowMenu.kind}
          repoRoot={repoRoot}
          selectionCount={treeSelection.selected.size}
          clipboardEmpty={clipboard.clipboard === null}
          onClose={() => { setRowMenu(null); }}
          onCut={() => { clipboard.cut(topmostPaths(treeSelection.selected)); }}
          onCopy={() => { clipboard.copy(topmostPaths(treeSelection.selected)); }}
          onPaste={handlePaste}
          onRename={() => { setRenamingPath(rowMenu.path); }}
          onDelete={() => { setPendingDelete(topmostPaths(treeSelection.selected)); }}
        />
      )}

      {blankMenu && (
        <ContextMenu
          x={blankMenu.x}
          y={blankMenu.y}
          items={[{
            label: "Paste",
            disabled: clipboard.clipboard === null,
            onClick: () => { handlePaste(""); },
          }]}
          onClose={() => { setBlankMenu(null); }}
        />
      )}

      {pendingDelete && (
        <ConfirmModal
          title={`Delete ${pendingDelete.length} item${pendingDelete.length === 1 ? "" : "s"}?`}
          message="Kept in the repo's quarantine until the next agent start."
          confirmLabel="Delete"
          isDestructive
          onConfirm={() => {
            actions.deleteEntries(pendingDelete);
            setPendingDelete(null);
          }}
          onCancel={() => { setPendingDelete(null); }}
        />
      )}

      <BundleDragGhost paths={rowDrag.draggedPaths} position={rowDrag.ghostPosition} />

      <BundleEntriesFeedback
        label="IMPORT"
        pendingReplace={importReplace}
        error={importError}
        onConfirmReplace={confirmImport}
        onCancelReplace={cancelImport}
        onDismissError={dismissImportError}
      />
      <BundleEntriesFeedback
        label="ACTION"
        pendingReplace={actions.pendingReplace}
        error={actions.error}
        onConfirmReplace={actions.confirmReplace}
        onCancelReplace={actions.cancelReplace}
        onDismissError={actions.dismissError}
      />
    </div>
  );
}
