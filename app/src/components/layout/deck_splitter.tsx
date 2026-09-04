// Path: app/src/components/layout/deck_splitter.tsx
// Description: Drag divider between the deck's left panel and the rail; previews the rail width while dragging and commits it on release

import type React from "react";
import { useCallback, useRef } from "react";
import "../../styles/deck_splitter.css";

export const RAIL_WIDTH_MIN_PERCENT = 20;
export const RAIL_WIDTH_MAX_PERCENT = 70;
export const RAIL_WIDTH_DEFAULT_PERCENT = 35;
const KEYBOARD_STEP_PERCENT = 2;

export function clampRailWidthPercent(percent: number): number {
  const rounded = Math.round(percent);
  return Math.min(RAIL_WIDTH_MAX_PERCENT, Math.max(RAIL_WIDTH_MIN_PERCENT, rounded));
}

interface DeckSplitterProps {
  /** The grid whose right column the rail occupies; the drag is measured against it */
  gridRef: React.RefObject<HTMLElement | null>;
  percent: number;
  /** Live width while dragging (no persistence) */
  onPreview: (percent: number) => void;
  /** Final width on release, keyboard step, or double-click reset */
  onCommit: (percent: number) => void;
}

/** Pointer position → rail share of the grid width, clamped */
function percentAt(grid: HTMLElement, clientX: number): number {
  const rect = grid.getBoundingClientRect();
  if (rect.width <= 0) return RAIL_WIDTH_DEFAULT_PERCENT;
  return clampRailWidthPercent(((rect.right - clientX) / rect.width) * 100);
}

export function DeckSplitter({
  gridRef,
  percent,
  onPreview,
  onCommit,
}: DeckSplitterProps): React.JSX.Element {
  const dragRef = useRef<{ pointerId: number; last: number } | null>(null);

  const handlePointerDown = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      if (event.button !== 0) return;
      event.preventDefault();
      event.currentTarget.setPointerCapture(event.pointerId);
      dragRef.current = { pointerId: event.pointerId, last: percent };
      event.currentTarget.dataset.dragging = "";
    },
    [percent]
  );

  const handlePointerMove = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      const drag = dragRef.current;
      const grid = gridRef.current;
      if (drag === null || drag.pointerId !== event.pointerId || grid === null) return;
      const next = percentAt(grid, event.clientX);
      if (next === drag.last) return;
      drag.last = next;
      onPreview(next);
    },
    [gridRef, onPreview]
  );

  const handlePointerEnd = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      const drag = dragRef.current;
      if (drag === null || drag.pointerId !== event.pointerId) return;
      dragRef.current = null;
      delete event.currentTarget.dataset.dragging;
      if (event.currentTarget.hasPointerCapture(event.pointerId)) {
        event.currentTarget.releasePointerCapture(event.pointerId);
      }
      onCommit(drag.last);
    },
    [onCommit]
  );

  const handleKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLDivElement>) => {
      // The rail sits on the right: Left grows it, Right shrinks it
      const delta =
        event.key === "ArrowLeft" ? KEYBOARD_STEP_PERCENT
        : event.key === "ArrowRight" ? -KEYBOARD_STEP_PERCENT
        : event.key === "Home" ? RAIL_WIDTH_DEFAULT_PERCENT - percent
        : null;
      if (delta === null) return;
      event.preventDefault();
      onCommit(clampRailWidthPercent(percent + delta));
    },
    [onCommit, percent]
  );

  return (
    <div
      className="deck-splitter"
      role="separator"
      aria-orientation="vertical"
      aria-label="Rail width"
      aria-valuemin={RAIL_WIDTH_MIN_PERCENT}
      aria-valuemax={RAIL_WIDTH_MAX_PERCENT}
      aria-valuenow={percent}
      tabIndex={0}
      title="Drag to resize the rail · double-click to reset"
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerEnd}
      onPointerCancel={handlePointerEnd}
      onDoubleClick={() => { onCommit(RAIL_WIDTH_DEFAULT_PERCENT); }}
      onKeyDown={handleKeyDown}
    >
      <span className="deck-splitter__grip" aria-hidden="true" />
    </div>
  );
}
