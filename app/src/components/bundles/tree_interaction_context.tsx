// Path: app/src/components/bundles/tree_interaction_context.tsx
// Description: Owns the ZIPS-tree click model (select/toggle/range/expand) and hands each row its interaction props

import type React from "react";
import { createContext, useCallback, useContext, useMemo, type ReactNode } from "react";
import type { VisibleRow } from "../../lib/bundles/flatten_visible_tree.js";

export type TreeRowKind = "dir" | "file";

export interface TreeRowMenuRequest {
  path: string;
  kind: TreeRowKind;
  x: number;
  y: number;
}

export interface RowInteraction {
  selected: boolean;
  cut: boolean;
  isDropTarget: boolean;
  renaming: boolean;
  onClick: (event: React.MouseEvent) => void;
  onContextMenu: (event: React.MouseEvent) => void;
  onPointerDown: (event: React.PointerEvent) => void;
  /** Only meaningful while `renaming` is true: commits or cancels the in-place rename input. */
  onRenameCommit: (newName: string) => void;
  onRenameCancel: () => void;
}

interface TreeInteractionContextValue {
  selected: ReadonlySet<string>;
  cutPaths: ReadonlySet<string>;
  dropTargetDir: string | null;
  renaming: string | null;
  handleRowClick: (path: string, kind: TreeRowKind, event: React.MouseEvent) => void;
  handleRowContextMenu: (path: string, kind: TreeRowKind, event: React.MouseEvent) => void;
  handleRowPointerDown: (path: string, kind: TreeRowKind, event: React.PointerEvent) => void;
  onRenameCommit: (newName: string) => void;
  onRenameCancel: () => void;
}

const noop = (): void => undefined;

/** Rows stay mountable outside the provider (e.g. in isolated tests), so the default is inert. */
const DEFAULT_CONTEXT: TreeInteractionContextValue = {
  selected: new Set(),
  cutPaths: new Set(),
  dropTargetDir: null,
  renaming: null,
  handleRowClick: noop,
  handleRowContextMenu: noop,
  handleRowPointerDown: noop,
  onRenameCommit: noop,
  onRenameCancel: noop,
};

const TreeInteractionContext = createContext<TreeInteractionContextValue>(DEFAULT_CONTEXT);

function isFromControl(event: { target: EventTarget }): boolean {
  return event.target instanceof HTMLElement && event.target.closest("button, input, label") !== null;
}

export interface TreeInteractionProviderProps {
  selected: ReadonlySet<string>;
  cutPaths: ReadonlySet<string>;
  dropTargetDir: string | null;
  renaming: string | null;
  visibleRows: readonly VisibleRow[];
  expandedDirs: ReadonlySet<string>;
  selectOnly: (path: string) => void;
  toggle: (path: string) => void;
  rangeTo: (path: string, visibleRows: readonly VisibleRow[]) => void;
  toggleExpanded: (path: string) => void;
  onStartDrag: (path: string, event: React.PointerEvent) => void;
  onOpenMenu: (request: TreeRowMenuRequest) => void;
  onRenameCommit: (newName: string) => void;
  onRenameCancel: () => void;
  children: ReactNode;
}

export function TreeInteractionProvider({
  selected,
  cutPaths,
  dropTargetDir,
  renaming,
  visibleRows,
  toggleExpanded,
  selectOnly,
  toggle,
  rangeTo,
  onStartDrag,
  onOpenMenu,
  onRenameCommit,
  onRenameCancel,
  children,
}: TreeInteractionProviderProps): React.JSX.Element {
  const handleRowClick = useCallback(
    (path: string, kind: TreeRowKind, event: React.MouseEvent): void => {
      if (isFromControl(event)) return;
      if (event.ctrlKey || event.metaKey) {
        toggle(path);
        return;
      }
      if (event.shiftKey) {
        rangeTo(path, visibleRows);
        return;
      }
      selectOnly(path);
      if (kind === "dir") toggleExpanded(path);
    },
    [rangeTo, selectOnly, toggle, toggleExpanded, visibleRows]
  );

  const handleRowContextMenu = useCallback(
    (path: string, kind: TreeRowKind, event: React.MouseEvent): void => {
      event.preventDefault();
      event.stopPropagation();
      if (!selected.has(path)) selectOnly(path);
      onOpenMenu({ path, kind, x: event.clientX, y: event.clientY });
    },
    [onOpenMenu, selectOnly, selected]
  );

  const handleRowPointerDown = useCallback(
    (path: string, _kind: TreeRowKind, event: React.PointerEvent): void => {
      if (event.button !== 0 || isFromControl(event)) return;
      onStartDrag(path, event);
    },
    [onStartDrag]
  );

  const value = useMemo<TreeInteractionContextValue>(
    () => ({
      selected,
      cutPaths,
      dropTargetDir,
      renaming,
      handleRowClick,
      handleRowContextMenu,
      handleRowPointerDown,
      onRenameCommit,
      onRenameCancel,
    }),
    [
      selected, cutPaths, dropTargetDir, renaming, handleRowClick, handleRowContextMenu,
      handleRowPointerDown, onRenameCommit, onRenameCancel,
    ]
  );

  return (
    <TreeInteractionContext.Provider value={value}>{children}</TreeInteractionContext.Provider>
  );
}

export function useRowInteraction(path: string, kind: TreeRowKind): RowInteraction {
  const ctx = useContext(TreeInteractionContext);
  return {
    selected: ctx.selected.has(path),
    cut: ctx.cutPaths.has(path),
    isDropTarget: ctx.dropTargetDir === path,
    renaming: ctx.renaming === path,
    onClick: (event) => { ctx.handleRowClick(path, kind, event); },
    onContextMenu: (event) => { ctx.handleRowContextMenu(path, kind, event); },
    onPointerDown: (event) => { ctx.handleRowPointerDown(path, kind, event); },
    onRenameCommit: ctx.onRenameCommit,
    onRenameCancel: ctx.onRenameCancel,
  };
}

/** The list container's own drop-target/root state, for the blank-area (path "") highlight. */
export function useRootDropTarget(): boolean {
  return useContext(TreeInteractionContext).dropTargetDir === "";
}
