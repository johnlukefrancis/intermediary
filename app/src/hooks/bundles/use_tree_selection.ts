// Path: app/src/hooks/bundles/use_tree_selection.ts
// Description: Row selection state for the ZIPS tree — click/ctrl/shift semantics, pruned as rows disappear

import { useCallback, useEffect, useRef, useState } from "react";
import type { VisibleRow } from "../../lib/bundles/flatten_visible_tree.js";
import { parentPath } from "../../lib/bundles/bundle_selection_visibility.js";
import type { DirectoryListingState } from "./use_directory_listings.js";

export interface TreeSelectionState {
  selected: ReadonlySet<string>;
  anchor: string | null;
  /** Plain click: this row only, and it becomes the shift-range anchor. */
  selectOnly: (path: string) => void;
  /** Ctrl-click: toggles this row's membership; it becomes the shift-range anchor. */
  toggle: (path: string) => void;
  /** Shift-click / Shift+Up/Down: selects the visible-order range from the anchor to this row. */
  rangeTo: (path: string, visibleRows: readonly VisibleRow[]) => void;
  clear: () => void;
  /** Sets the selection to exactly these paths (e.g. the new path after a rename). */
  replaceWith: (paths: readonly string[]) => void;
}

interface UseTreeSelectionOptions {
  repoId: string;
  visibleRows: readonly VisibleRow[];
  listings: ReadonlyMap<string, DirectoryListingState>;
}

export function useTreeSelection({
  repoId,
  visibleRows,
  listings,
}: UseTreeSelectionOptions): TreeSelectionState {
  const [selected, setSelected] = useState<Set<string>>(() => new Set());
  const [anchor, setAnchor] = useState<string | null>(null);
  const selectedRef = useRef(selected);
  selectedRef.current = selected;

  useEffect(() => {
    setSelected(new Set());
    setAnchor(null);
  }, [repoId]);

  const selectOnly = useCallback((path: string): void => {
    setSelected(new Set([path]));
    setAnchor(path);
  }, []);

  const toggle = useCallback((path: string): void => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
    setAnchor(path);
  }, []);

  const rangeTo = useCallback(
    (path: string, rows: readonly VisibleRow[]): void => {
      if (anchor === null) {
        selectOnly(path);
        return;
      }
      const anchorIndex = rows.findIndex((row) => row.path === anchor);
      const targetIndex = rows.findIndex((row) => row.path === path);
      if (anchorIndex === -1 || targetIndex === -1) {
        selectOnly(path);
        return;
      }
      const [start, end] = anchorIndex <= targetIndex
        ? [anchorIndex, targetIndex]
        : [targetIndex, anchorIndex];
      setSelected(new Set(rows.slice(start, end + 1).map((row) => row.path)));
    },
    [anchor, selectOnly]
  );

  const clear = useCallback((): void => {
    setSelected(new Set());
    setAnchor(null);
  }, []);

  const replaceWith = useCallback((paths: readonly string[]): void => {
    setSelected(new Set(paths));
    setAnchor(paths.length > 0 ? paths[paths.length - 1] ?? null : null);
  }, []);

  // Prune once a fresh, confirmed listing shows a selected path is gone. A path merely hidden by
  // a collapsed or not-yet-loaded ancestor is left alone: only a *ready* immediate parent listing
  // (or the always-current root) counts as confirmation that the path no longer exists.
  useEffect(() => {
    const current = selectedRef.current;
    if (current.size === 0) return;
    const visiblePaths = new Set(visibleRows.map((row) => row.path));
    let changed = false;
    const next = new Set<string>();
    for (const path of current) {
      if (visiblePaths.has(path)) {
        next.add(path);
        continue;
      }
      const parent = parentPath(path);
      const parentReady = parent === "" || listings.get(parent)?.status === "ready";
      if (parentReady) {
        changed = true;
        continue;
      }
      next.add(path);
    }
    if (changed) setSelected(next);
  }, [visibleRows, listings]);

  return { selected, anchor, selectOnly, toggle, rangeTo, clear, replaceWith };
}
