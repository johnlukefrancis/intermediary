// Path: app/src/hooks/bundles/use_tree_keyboard.ts
// Description: Keyboard command map for the focused ZIPS tree list (navigation, expand/collapse, clipboard, rename, delete)

import { useCallback } from "react";
import type React from "react";
import type { VisibleRow } from "../../lib/bundles/flatten_visible_tree.js";
import { parentPath, topmostPaths } from "../../lib/bundles/bundle_selection_visibility.js";

export interface UseTreeKeyboardOptions {
  visibleRows: readonly VisibleRow[];
  selected: ReadonlySet<string>;
  anchor: string | null;
  expandedDirs: ReadonlySet<string>;
  /** Non-null while a row's name is being edited; Escape cancels the rename instead of the selection. */
  renaming: string | null;
  selectOnly: (path: string) => void;
  rangeTo: (path: string, visibleRows: readonly VisibleRow[]) => void;
  clearSelection: () => void;
  expandDirectory: (path: string) => void;
  toggleExpanded: (path: string) => void;
  onOpenFile: (path: string) => void;
  onDeleteRequest: (paths: string[]) => void;
  onRenameStart: (path: string) => void;
  onRenameCancel: () => void;
  onCut: (paths: string[]) => void;
  onCopy: (paths: string[]) => void;
  onPaste: () => void;
}

export interface TreeKeyboardHandlers {
  onKeyDown: (event: React.KeyboardEvent) => void;
}

function isTextEntryTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName.toLowerCase();
  return tag === "input" || tag === "textarea";
}

export function useTreeKeyboard({
  visibleRows,
  selected,
  anchor,
  expandedDirs,
  renaming,
  selectOnly,
  rangeTo,
  clearSelection,
  expandDirectory,
  toggleExpanded,
  onOpenFile,
  onDeleteRequest,
  onRenameStart,
  onRenameCancel,
  onCut,
  onCopy,
  onPaste,
}: UseTreeKeyboardOptions): TreeKeyboardHandlers {
  const onKeyDown = useCallback(
    (event: React.KeyboardEvent): void => {
      if (document.querySelector("[data-intermediary-modal-root]") !== null) return;
      if (isTextEntryTarget(event.target)) return;

      const currentPath = anchor ?? (selected.size > 0 ? [...selected][0] : null) ?? null;
      const currentIndex = currentPath === null
        ? -1
        : visibleRows.findIndex((row) => row.path === currentPath);

      switch (true) {
        case event.key === "ArrowDown" || event.key === "ArrowUp": {
          event.preventDefault();
          if (visibleRows.length === 0) return;
          const delta = event.key === "ArrowDown" ? 1 : -1;
          const nextIndex = currentIndex === -1
            ? (delta === 1 ? 0 : visibleRows.length - 1)
            : Math.min(Math.max(currentIndex + delta, 0), visibleRows.length - 1);
          const nextPath = visibleRows[nextIndex]?.path;
          if (nextPath === undefined) return;
          if (event.shiftKey) {
            rangeTo(nextPath, visibleRows);
          } else {
            selectOnly(nextPath);
          }
          return;
        }
        case event.key === "ArrowRight": {
          if (currentPath === null) return;
          const row = visibleRows[currentIndex];
          if (!row || row.kind !== "dir") return;
          event.preventDefault();
          if (!expandedDirs.has(currentPath)) expandDirectory(currentPath);
          return;
        }
        case event.key === "ArrowLeft": {
          if (currentPath === null) return;
          const row = visibleRows[currentIndex];
          if (!row) return;
          event.preventDefault();
          if (row.kind === "dir" && expandedDirs.has(currentPath)) {
            toggleExpanded(currentPath);
            return;
          }
          const parent = parentPath(currentPath);
          if (parent !== "") selectOnly(parent);
          return;
        }
        case event.key === "Enter": {
          if (currentPath === null) return;
          const row = visibleRows[currentIndex];
          if (!row) return;
          event.preventDefault();
          if (row.kind === "file") {
            onOpenFile(currentPath);
          } else {
            toggleExpanded(currentPath);
          }
          return;
        }
        case event.key === "Delete": {
          if (selected.size === 0) return;
          event.preventDefault();
          onDeleteRequest(topmostPaths(selected));
          return;
        }
        case event.key === "F2": {
          if (selected.size !== 1) return;
          event.preventDefault();
          onRenameStart([...selected][0] as string);
          return;
        }
        case event.ctrlKey && (event.key === "x" || event.key === "X"): {
          if (selected.size === 0) return;
          event.preventDefault();
          onCut(topmostPaths(selected));
          return;
        }
        case event.ctrlKey && (event.key === "c" || event.key === "C"): {
          if (selected.size === 0) return;
          event.preventDefault();
          onCopy(topmostPaths(selected));
          return;
        }
        case event.ctrlKey && (event.key === "v" || event.key === "V"): {
          event.preventDefault();
          onPaste();
          return;
        }
        case event.key === "Escape": {
          event.preventDefault();
          if (renaming !== null) {
            onRenameCancel();
          } else {
            clearSelection();
          }
          return;
        }
        default:
          return;
      }
    },
    [
      anchor, selected, visibleRows, expandedDirs, renaming, selectOnly, rangeTo, clearSelection,
      expandDirectory, toggleExpanded, onOpenFile, onDeleteRequest, onRenameStart, onRenameCancel,
      onCut, onCopy, onPaste,
    ]
  );

  return { onKeyDown };
}
