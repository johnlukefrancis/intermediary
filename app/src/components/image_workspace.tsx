// Path: app/src/components/image_workspace.tsx
// Description: Fit-to-panel image preview surface for shared repo workspaces

import type React from "react";
import { useDragOutPointer } from "../hooks/use_drag_out_pointer.js";
import { useImageBlobUrl } from "../hooks/use_image_blob_url.js";

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
  const pointer = useDragOutPointer({ onDragStart, enabled: source.status === "ready" });

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
      {...pointer}
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
