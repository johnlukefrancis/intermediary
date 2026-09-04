// Path: app/src/hooks/bundles/use_tree_row_drag.ts
// Description: In-tree pointer-drag of a row (or the whole selection) onto a folder row or the root

import { topmostPaths } from "../../lib/bundles/bundle_selection_visibility.js";
import { useCallback, useEffect, useRef, useState, type RefObject } from "react";
import type React from "react";
import { applyEdgeAutoScroll, resolveDropDir, useDropTargetDwell } from "./tree_drop_targeting.js";
import { parentPath } from "../../lib/bundles/bundle_selection_visibility.js";

const DRAG_START_DISTANCE_PX = 6;

export interface UseTreeRowDragOptions {
  listRef: RefObject<HTMLDivElement>;
  selected: ReadonlySet<string>;
  expandedDirs: ReadonlySet<string>;
  expandDirectory: (path: string) => void;
  onDrop: (paths: string[], directory: string) => void;
}

export interface TreeRowDragState {
  dropTargetDir: string | null;
  isDragging: boolean;
  draggedPaths: readonly string[];
  ghostPosition: { x: number; y: number } | null;
  startDrag: (path: string, event: React.PointerEvent) => void;
}

interface DragStart {
  pointerId: number;
  x: number;
  y: number;
  path: string;
}

/** A drop target that is the dragged item itself, its descendant, or every dragged entry's current parent. */
function isInvalidTarget(target: string, dragSet: readonly string[]): boolean {
  for (const path of dragSet) {
    if (target === path || target.startsWith(`${path}/`)) return true;
  }
  const parents = new Set(dragSet.map(parentPath));
  return parents.size === 1 && parents.has(target);
}

export function useTreeRowDrag({
  listRef,
  selected,
  expandedDirs,
  expandDirectory,
  onDrop,
}: UseTreeRowDragOptions): TreeRowDragState {
  const { dropTargetDir, setTarget, reset } = useDropTargetDwell({ expandedDirs, expandDirectory });
  const [isDragging, setIsDragging] = useState(false);
  const [draggedPaths, setDraggedPaths] = useState<string[]>([]);
  const [ghostPosition, setGhostPosition] = useState<{ x: number; y: number } | null>(null);

  const startInfoRef = useRef<DragStart | null>(null);
  const draggingRef = useRef(false);
  const dragSetRef = useRef<string[]>([]);
  const selectedRef = useRef(selected);
  selectedRef.current = selected;
  const onDropRef = useRef(onDrop);
  onDropRef.current = onDrop;

  const endGesture = useCallback((): void => {
    startInfoRef.current = null;
    draggingRef.current = false;
    dragSetRef.current = [];
    setIsDragging(false);
    setDraggedPaths([]);
    setGhostPosition(null);
    reset();
  }, [reset]);

  useEffect(() => {
    function handleMove(event: PointerEvent): void {
      const start = startInfoRef.current;
      if (start === null || event.pointerId !== start.pointerId) return;

      if (!draggingRef.current) {
        const distance = Math.hypot(event.clientX - start.x, event.clientY - start.y);
        if (distance < DRAG_START_DISTANCE_PX) return;
        const dragSet = selectedRef.current.has(start.path) ? topmostPaths(selectedRef.current) : [start.path];
        dragSetRef.current = dragSet;
        draggingRef.current = true;
        setIsDragging(true);
        setDraggedPaths(dragSet);
      }

      setGhostPosition({ x: event.clientX, y: event.clientY });
      const target = resolveDropDir(event.clientX, event.clientY);
      setTarget(target !== null && !isInvalidTarget(target, dragSetRef.current) ? target : null);
      applyEdgeAutoScroll(listRef, event.clientY);
    }

    function handleUp(event: PointerEvent): void {
      const start = startInfoRef.current;
      if (start === null || event.pointerId !== start.pointerId) return;
      if (draggingRef.current) {
        const target = resolveDropDir(event.clientX, event.clientY);
        if (target !== null && !isInvalidTarget(target, dragSetRef.current)) {
          onDropRef.current(dragSetRef.current, target);
        }
      }
      endGesture();
    }

    function handleCancel(event: PointerEvent): void {
      if (startInfoRef.current?.pointerId !== event.pointerId) return;
      endGesture();
    }

    function handleKeyDown(event: KeyboardEvent): void {
      if (event.key === "Escape" && startInfoRef.current !== null) endGesture();
    }

    window.addEventListener("pointermove", handleMove);
    window.addEventListener("pointerup", handleUp);
    window.addEventListener("pointercancel", handleCancel);
    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("pointermove", handleMove);
      window.removeEventListener("pointerup", handleUp);
      window.removeEventListener("pointercancel", handleCancel);
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [endGesture, listRef, setTarget]);

  const startDrag = useCallback((path: string, event: React.PointerEvent): void => {
    startInfoRef.current = { pointerId: event.pointerId, x: event.clientX, y: event.clientY, path };
  }, []);

  return { dropTargetDir, isDragging, draggedPaths, ghostPosition, startDrag };
}
