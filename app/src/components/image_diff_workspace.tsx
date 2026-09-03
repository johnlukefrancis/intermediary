// Path: app/src/components/image_diff_workspace.tsx
// Description: Side-by-side before/after viewer for a changed image opened from source control

import type React from "react";
import type { ImageDiffSide } from "../shared/protocol.js";
import { ImageDiffPane } from "./image_diff_pane.js";

const CONFLICT_NOTICE = "Merge conflict · keep one version, then stage it to mark it resolved";

interface ImageDiffWorkspaceViewerProps {
  path: string;
  isLoading: boolean;
  error: string | null;
  before: ImageDiffSide | null;
  after: ImageDiffSide | null;
  /** Unmerged path: the panes are stage 2 and stage 3, and the notice names the resolve step */
  conflict: boolean;
}

export function ImageDiffWorkspaceViewer({
  path,
  isLoading,
  error,
  before,
  after,
  conflict,
}: ImageDiffWorkspaceViewerProps): React.JSX.Element {
  if (isLoading) {
    return (
      <div className="image-diff-workspace">
        <p className="empty-state empty-state--waiting">Loading image diff</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="image-diff-workspace">
        <p className="text-workspace-error text-workspace-error--inline">{error}</p>
      </div>
    );
  }

  return (
    <div className="image-diff-workspace" role="region" aria-label={`Image diff for ${path}`}>
      {conflict && (
        <p className="image-diff-workspace__notice" role="alert">
          {CONFLICT_NOTICE}
        </p>
      )}
      {(before === null) !== (after === null) ? (
        // A one-sided change has nothing to compare: show the one image full width
        <div className="image-diff-workspace__panes" data-layout="single">
          {after === null ? (
            <ImageDiffPane path={path} slot="before" side={before} solo="deleted" />
          ) : (
            <ImageDiffPane path={path} slot="after" side={after} solo="new" />
          )}
        </div>
      ) : (
        <div className="image-diff-workspace__panes">
          <ImageDiffPane path={path} slot="before" side={before} />
          <ImageDiffPane path={path} slot="after" side={after} />
        </div>
      )}
    </div>
  );
}
