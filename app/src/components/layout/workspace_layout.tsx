// Path: app/src/components/layout/workspace_layout.tsx
// Description: Layout that replaces Auto files with a shared workspace

import type React from "react";
import { useCallback, useRef } from "react";
import "../../styles/handset_deck.css";
import "../../styles/handset_chassis.css";

const DRAG_START_DISTANCE_PX = 6;

interface WorkspaceLayoutProps {
  title: string;
  subtitle: string | undefined;
  /** `alert` renders the subtitle in the error tone (a merge-conflict diff) */
  subtitleTone?: "alert" | undefined;
  onClose: () => void;
  onTitleContextMenu?: ((event: React.MouseEvent) => void) | undefined;
  onTitleDragStart?: (() => void | Promise<void>) | undefined;
  content: React.ReactNode;
  railContent: React.ReactNode;
  isHandset: boolean;
}

interface WorkspacePanelProps {
  title: string;
  subtitle: string | undefined;
  subtitleTone?: "alert" | undefined;
  onClose: () => void;
  onTitleContextMenu?: ((event: React.MouseEvent) => void) | undefined;
  onTitleDragStart?: (() => void | Promise<void>) | undefined;
  content: React.ReactNode;
}

function WorkspacePanel({
  title,
  subtitle,
  subtitleTone,
  onClose,
  onTitleContextMenu,
  onTitleDragStart,
  content,
}: WorkspacePanelProps): React.JSX.Element {
  const dragStartRef = useRef<{ pointerId: number; x: number; y: number } | null>(null);

  const clearPointerCapture = useCallback((target: Element, pointerId: number): void => {
    if (!(target instanceof HTMLElement) || !target.hasPointerCapture(pointerId)) return;
    target.releasePointerCapture(pointerId);
  }, []);

  const handleTitlePointerDown = useCallback(
    (event: React.PointerEvent) => {
      if (event.button !== 0 || !onTitleDragStart) return;
      dragStartRef.current = {
        pointerId: event.pointerId,
        x: event.clientX,
        y: event.clientY,
      };
      event.currentTarget.setPointerCapture(event.pointerId);
    },
    [onTitleDragStart]
  );

  const handleTitlePointerMove = useCallback(
    (event: React.PointerEvent) => {
      const start = dragStartRef.current;
      if (!start || start.pointerId !== event.pointerId || !onTitleDragStart) return;
      if ((event.buttons & 1) !== 1) {
        clearPointerCapture(event.currentTarget, event.pointerId);
        dragStartRef.current = null;
        return;
      }

      const distance = Math.hypot(event.clientX - start.x, event.clientY - start.y);
      if (distance < DRAG_START_DISTANCE_PX) return;

      clearPointerCapture(event.currentTarget, event.pointerId);
      dragStartRef.current = null;
      void onTitleDragStart();
    },
    [clearPointerCapture, onTitleDragStart]
  );

  const handleTitlePointerEnd = useCallback(
    (event: React.PointerEvent) => {
      clearPointerCapture(event.currentTarget, event.pointerId);
      if (dragStartRef.current?.pointerId === event.pointerId) {
        dragStartRef.current = null;
      }
    },
    [clearPointerCapture]
  );

  return (
    <section className="panel text-workspace-panel">
      <header className="panel-header text-workspace-header">
        <div className="text-workspace-heading">
          <h2
            className="text-workspace-title"
            data-draggable={onTitleDragStart ? true : undefined}
            onPointerDown={handleTitlePointerDown}
            onPointerMove={handleTitlePointerMove}
            onPointerUp={handleTitlePointerEnd}
            onPointerCancel={handleTitlePointerEnd}
            onContextMenu={(event) => {
              if (!onTitleContextMenu) return;
              event.preventDefault();
              onTitleContextMenu(event);
            }}
            title={onTitleDragStart ? "Drag to attach; right-click for file actions" : title}
          >
            {title}
          </h2>
          {subtitle && (
            <span className="text-workspace-subtitle" data-tone={subtitleTone}>{subtitle}</span>
          )}
        </div>
        <button
          type="button"
          className="panel-header-icon text-workspace-close"
          onClick={onClose}
          title="Close workspace"
          aria-label="Close workspace"
        >
          ×
        </button>
      </header>
      <div className="panel-content text-workspace-content">{content}</div>
    </section>
  );
}

export function WorkspaceLayout({
  title,
  subtitle,
  subtitleTone,
  onClose,
  onTitleContextMenu,
  onTitleDragStart,
  content,
  railContent,
  isHandset,
}: WorkspaceLayoutProps): React.JSX.Element {
  const panel = (
    <WorkspacePanel
      title={title}
      subtitle={subtitle}
      subtitleTone={subtitleTone}
      onClose={onClose}
      onTitleContextMenu={onTitleContextMenu}
      onTitleDragStart={onTitleDragStart}
      content={content}
    />
  );

  // Handset with an open workspace stays workspace-only; close is the route back to the deck.
  if (isHandset) {
    return (
      <div className="handset-deck text-workspace-handset">
        <div className="handset-chassis">{panel}</div>
      </div>
    );
  }

  return (
    <div className="text-workspace-layout">
      {panel}
      {railContent}
    </div>
  );
}
