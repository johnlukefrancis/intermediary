// Path: app/src/hooks/use_file_selection.ts
// Description: Multi-file selection state hook with shift-range and ctrl/cmd-toggle support

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { FileEntry } from "../shared/protocol.js";

const IS_MAC = /Mac|iPhone|iPad|iPod/.test(navigator.userAgent);

function isToggleModifier(e: React.MouseEvent): boolean {
  return IS_MAC ? e.metaKey : e.ctrlKey;
}

export interface UseFileSelectionResult {
  selectedPaths: ReadonlySet<string>;
  selectionCount: number;
  isSelected: (path: string) => boolean;
  handleSelect: (path: string, event: React.MouseEvent) => void;
  clearSelection: () => void;
}

export function useFileSelection(files: FileEntry[]): UseFileSelectionResult {
  const [selectedPaths, setSelectedPaths] = useState<ReadonlySet<string>>(
    () => new Set<string>()
  );
  const anchorRef = useRef<string | null>(null);

  const orderedPaths = useMemo(() => files.map((f) => f.path), [files]);

  // Auto-prune: remove selected paths that are no longer in the file list
  useEffect(() => {
    setSelectedPaths((prev) => {
      if (prev.size === 0) return prev;
      const currentSet = new Set(orderedPaths);
      const pruned = new Set<string>();
      for (const p of prev) {
        if (currentSet.has(p)) pruned.add(p);
      }
      if (pruned.size === prev.size) return prev;
      return pruned;
    });
  }, [orderedPaths]);

  const handleSelect = useCallback(
    (path: string, event: React.MouseEvent) => {
      const toggle = isToggleModifier(event);
      const shift = event.shiftKey;

      if (shift && anchorRef.current !== null) {
        // Range select from anchor to clicked path
        const anchorIdx = orderedPaths.indexOf(anchorRef.current);
        const targetIdx = orderedPaths.indexOf(path);
        if (anchorIdx === -1 || targetIdx === -1) return;
        const start = Math.min(anchorIdx, targetIdx);
        const end = Math.max(anchorIdx, targetIdx);
        const range = orderedPaths.slice(start, end + 1);

        if (toggle) {
          // Shift+Ctrl/Cmd: add range to existing selection
          setSelectedPaths((prev) => {
            const next = new Set(prev);
            for (const p of range) next.add(p);
            return next;
          });
        } else {
          // Shift only: replace selection with range
          setSelectedPaths(new Set(range));
        }
      } else if (toggle) {
        // Ctrl/Cmd: toggle individual file
        anchorRef.current = path;
        setSelectedPaths((prev) => {
          const next = new Set(prev);
          if (next.has(path)) {
            next.delete(path);
          } else {
            next.add(path);
          }
          return next;
        });
      } else {
        // No modifier with shift but no anchor — start fresh selection
        anchorRef.current = path;
        setSelectedPaths(new Set([path]));
      }
    },
    [orderedPaths]
  );

  const clearSelection = useCallback(() => {
    setSelectedPaths(new Set<string>());
    anchorRef.current = null;
  }, []);

  const isSelected = useCallback(
    (path: string): boolean => selectedPaths.has(path),
    [selectedPaths]
  );

  return {
    selectedPaths,
    selectionCount: selectedPaths.size,
    isSelected,
    handleSelect,
    clearSelection,
  };
}
