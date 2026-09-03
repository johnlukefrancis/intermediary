// Path: app/src/components/image_workspace.tsx
// Description: Fit-to-panel image preview surface for shared repo workspaces

import type React from "react";
import { useCallback, useRef } from "react";
import { useImageBlobUrl } from "../hooks/use_image_blob_url.js";

const DRAG_START_DISTANCE_PX = 6;

interface ImageWorkspaceViewerProps {
  path: string;
  dataBase64: string | undefined;
  mimeType: string | undefined;
  isLoading: boolean;
  error: string | null;
  onDragStart: () => void | Promise<void>;
}

export function ImageWorkspaceViewer({
  path,
  dataBase64,
  mimeType,
  isLoading,
  error,
  onDragStart,
}: ImageWorkspaceViewerProps): React.JSX.Element {
  const source = useImageBlobUrl(dataBase64, mimeType);
  const dragStartRef = useRef<{
    pointerId: number;
    x: number;
    y: number;
  } | null>(null);

  const clearPointerCapture = useCallback((target: Element, pointerId: number): void => {
    if (!(target instanceof HTMLElement) || !target.hasPointerCapture(pointerId)) return;
    target.releasePointerCapture(pointerId);
  }, []);

  const handlePointerDown = useCallback(
    (event: React.PointerEvent) => {
      if (event.button !== 0 || source.status !== "ready") return;
      dragStartRef.current = {
        pointerId: event.pointerId,
        x: event.clientX,
        y: event.clientY,
      };
      event.currentTarget.setPointerCapture(event.pointerId);
    },
    [source.status]
  );

  const handlePointerMove = useCallback(
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
    [clearPointerCapture, onDragStart]
  );

  const handlePointerEnd = useCallback(
    (event: React.PointerEvent) => {
      clearPointerCapture(event.currentTarget, event.pointerId);
      if (dragStartRef.current?.pointerId === event.pointerId) {
        dragStartRef.current = null;
      }
    },
    [clearPointerCapture]
  );

  if (isLoading) {
    return <p className="empty-state empty-state--waiting">Loading image</p>;
  }

  if (error) {
    return <p className="text-workspace-error text-workspace-error--inline">{error}</p>;
  }

  if (source.status === "error") {
    return <p className="text-workspace-error text-workspace-error--inline">{source.message}</p>;
  }

  if (source.status !== "ready") {
    return <p className="empty-state empty-state--waiting">Preparing image</p>;
  }

  return (
    <div
      className="text-workspace-image-shell"
      data-draggable="true"
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerEnd}
      onPointerCancel={handlePointerEnd}
      title="Drag image to attach elsewhere"
    >
      <img
        className="text-workspace-image"
        src={source.url}
        alt={path}
        draggable={false}
        onDragStart={(event) => {
          event.preventDefault();
        }}
      />
    </div>
  );
}
