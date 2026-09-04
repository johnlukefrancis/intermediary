// Path: app/src/hooks/bundles/tree_drop_targeting.ts
// Description: Shared drop-target hit-testing, dwell-to-expand, and edge auto-scroll for the ZIPS tree
//
// Contract: every function here takes CSS px, never physical px. The OS drag-drop hook divides
// `event.payload.position` by `devicePixelRatio` before calling in; pointer events (drag-move,
// click) are already CSS px and need no conversion.

import { useCallback, useEffect, useRef, useState, type RefObject } from "react";

const DWELL_EXPAND_MS = 700;
const EDGE_ZONE_PX = 28;
const EDGE_SCROLL_PX = 12;

/** The repo-relative directory at a viewport point (CSS px), or null when nothing under it is a target. */
export function resolveDropDir(cssX: number, cssY: number): string | null {
  const element = document.elementFromPoint(cssX, cssY);
  const dirElement = element?.closest<HTMLElement>("[data-drop-dir]");
  const value = dirElement?.dataset.dropDir;
  return value === undefined ? null : value;
}

/** Nudges the list's scroll position (CSS px) when the pointer sits near its top/bottom edge. */
export function applyEdgeAutoScroll(listRef: RefObject<HTMLDivElement>, cssY: number): void {
  const list = listRef.current;
  if (!list) return;
  const rect = list.getBoundingClientRect();
  if (cssY - rect.top < EDGE_ZONE_PX) {
    list.scrollTop -= EDGE_SCROLL_PX;
  } else if (rect.bottom - cssY < EDGE_ZONE_PX) {
    list.scrollTop += EDGE_SCROLL_PX;
  }
}

export interface UseDropTargetDwellOptions {
  expandedDirs: ReadonlySet<string>;
  expandDirectory: (path: string) => void;
}

export interface DropTargetDwell {
  dropTargetDir: string | null;
  /** Sets the current target; re-arms the dwell timer only when the target actually changes. */
  setTarget: (next: string | null) => void;
  /** Clears the target and cancels any pending dwell timer. */
  reset: () => void;
}

/**
 * Owns the 700 ms hover-to-expand timer shared by the OS drag-drop hook and the in-tree row drag.
 * The timer never arms for the root ("") since the root is always expanded.
 */
export function useDropTargetDwell({
  expandedDirs,
  expandDirectory,
}: UseDropTargetDwellOptions): DropTargetDwell {
  const [dropTargetDir, setDropTargetDir] = useState<string | null>(null);
  const dropTargetRef = useRef<string | null>(null);
  const dwellTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const expandedDirsRef = useRef(expandedDirs);
  const expandDirectoryRef = useRef(expandDirectory);

  useEffect(() => { expandedDirsRef.current = expandedDirs; }, [expandedDirs]);
  useEffect(() => { expandDirectoryRef.current = expandDirectory; }, [expandDirectory]);

  const clearDwellTimer = useCallback((): void => {
    if (dwellTimerRef.current === null) return;
    clearTimeout(dwellTimerRef.current);
    dwellTimerRef.current = null;
  }, []);

  const setTarget = useCallback(
    (next: string | null): void => {
      if (dropTargetRef.current === next) return;
      dropTargetRef.current = next;
      clearDwellTimer();
      setDropTargetDir(next);
      if (next !== null && next !== "" && !expandedDirsRef.current.has(next)) {
        dwellTimerRef.current = setTimeout(() => {
          dwellTimerRef.current = null;
          expandDirectoryRef.current(next);
        }, DWELL_EXPAND_MS);
      }
    },
    [clearDwellTimer]
  );

  const reset = useCallback((): void => {
    dropTargetRef.current = null;
    clearDwellTimer();
    setDropTargetDir(null);
  }, [clearDwellTimer]);

  useEffect(() => clearDwellTimer, [clearDwellTimer]);

  return { dropTargetDir, setTarget, reset };
}
