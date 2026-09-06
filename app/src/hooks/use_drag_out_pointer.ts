// Path: app/src/hooks/use_drag_out_pointer.ts
// Description: Pointer handlers for the 6 px drag-out threshold with pointer capture, shared by rows, images, and stream cards

import type React from "react";
import { useCallback, useRef } from "react";

const DRAG_START_DISTANCE_PX = 6;

type ModifierEvent = Pick<React.MouseEvent, "ctrlKey" | "metaKey" | "shiftKey">;

interface UseDragOutPointerOptions {
  onDragStart: () => void | Promise<void>;
  /** When given, a Ctrl/Shift/Meta press selects instead of arming a drag */
  onSelect?: ((event: ModifierEvent) => void) | undefined;
  /** False disables arming (e.g. while an image is still loading) */
  enabled?: boolean | undefined;
}

export interface DragOutPointerHandlers {
  onPointerDown: (event: React.PointerEvent) => void;
  onPointerMove: (event: React.PointerEvent) => void;
  onPointerUp: (event: React.PointerEvent) => void;
  onPointerCancel: (event: React.PointerEvent) => void;
}

function clearPointerCapture(target: Element, pointerId: number): void {
  if (!(target instanceof HTMLElement) || !target.hasPointerCapture(pointerId)) return;
  target.releasePointerCapture(pointerId);
}

/**
 * A primary-button press arms a drag; moving past the threshold while the button is held
 * releases capture and starts the drag-out exactly once. Presses on nested buttons are ignored
 * so a row's own controls keep their click.
 */
export function useDragOutPointer({
  onDragStart,
  onSelect,
  enabled = true,
}: UseDragOutPointerOptions): DragOutPointerHandlers {
  const dragStartRef = useRef<{ pointerId: number; x: number; y: number } | null>(null);

  const onPointerDown = useCallback(
    (event: React.PointerEvent) => {
      if (event.button !== 0 || !enabled) return;
      if (event.target instanceof Element && event.target.closest("button")) return;

      if (onSelect && (event.shiftKey || event.metaKey || event.ctrlKey)) {
        event.preventDefault();
        onSelect(event);
        return;
      }

      dragStartRef.current = { pointerId: event.pointerId, x: event.clientX, y: event.clientY };
      event.currentTarget.setPointerCapture(event.pointerId);
    },
    [enabled, onSelect]
  );

  const onPointerMove = useCallback(
    (event: React.PointerEvent) => {
      const start = dragStartRef.current;
      if (!start || start.pointerId !== event.pointerId) return;
      if ((event.buttons & 1) !== 1) {
        clearPointerCapture(event.currentTarget, event.pointerId);
        dragStartRef.current = null;
        return;
      }

      const distance = Math.hypot(event.clientX - start.x, event.clientY - start.y);
      if (distance < DRAG_START_DISTANCE_PX) return;

      clearPointerCapture(event.currentTarget, event.pointerId);
      dragStartRef.current = null;
      void onDragStart();
    },
    [onDragStart]
  );

  const onPointerEnd = useCallback((event: React.PointerEvent) => {
    clearPointerCapture(event.currentTarget, event.pointerId);
    if (dragStartRef.current?.pointerId === event.pointerId) {
      dragStartRef.current = null;
    }
  }, []);

  return { onPointerDown, onPointerMove, onPointerUp: onPointerEnd, onPointerCancel: onPointerEnd };
}
