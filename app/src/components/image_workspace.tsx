// Path: app/src/components/image_workspace.tsx
// Description: Fit-to-panel image preview surface for shared repo workspaces

import type React from "react";
import { useCallback, useEffect, useRef, useState } from "react";

const DRAG_START_DISTANCE_PX = 6;

interface ImageWorkspaceViewerProps {
  path: string;
  dataBase64: string | undefined;
  mimeType: string | undefined;
  isLoading: boolean;
  error: string | null;
  onDragStart: () => void | Promise<void>;
}

type ImageSourceState =
  | { kind: "none" }
  | { kind: "ready"; url: string }
  | { kind: "error"; message: string };

function base64ToBlob(dataBase64: string, mimeType: string): Blob {
  const binary = globalThis.atob(dataBase64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i);
  }
  return new Blob([bytes], { type: mimeType });
}

export function ImageWorkspaceViewer({
  path,
  dataBase64,
  mimeType,
  isLoading,
  error,
  onDragStart,
}: ImageWorkspaceViewerProps): React.JSX.Element {
  const [source, setSource] = useState<ImageSourceState>({ kind: "none" });
  const dragStartRef = useRef<{
    pointerId: number;
    x: number;
    y: number;
  } | null>(null);

  useEffect(() => {
    if (!dataBase64 || !mimeType) {
      setSource({ kind: "none" });
      return undefined;
    }

    let url: string;
    try {
      url = URL.createObjectURL(base64ToBlob(dataBase64, mimeType));
    } catch (err) {
      const message = err instanceof Error ? err.message : "Unable to decode image preview";
      setSource({ kind: "error", message });
      return undefined;
    }

    setSource({ kind: "ready", url });
    return () => {
      URL.revokeObjectURL(url);
    };
  }, [dataBase64, mimeType]);

  const clearPointerCapture = useCallback((target: Element, pointerId: number): void => {
    if (!(target instanceof HTMLElement) || !target.hasPointerCapture(pointerId)) return;
    target.releasePointerCapture(pointerId);
  }, []);

  const handlePointerDown = useCallback(
    (event: React.PointerEvent) => {
      if (event.button !== 0 || source.kind !== "ready") return;
      dragStartRef.current = {
        pointerId: event.pointerId,
        x: event.clientX,
        y: event.clientY,
      };
      event.currentTarget.setPointerCapture(event.pointerId);
    },
    [source.kind]
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

  if (source.kind === "error") {
    return <p className="text-workspace-error text-workspace-error--inline">{source.message}</p>;
  }

  if (source.kind !== "ready") {
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
