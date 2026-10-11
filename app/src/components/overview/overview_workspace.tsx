// Path: app/src/components/overview/overview_workspace.tsx
// Description: Overview's full-pane file or diff workspace using the shared repository viewers and actions

import type React from "react";
import { useCallback, useEffect, useRef } from "react";
import type { RepoConfig } from "../../shared/config.js";
import type { SourceControlEntry } from "../../shared/protocol.js";
import { useRepoWorkspace } from "../../hooks/use_repo_workspace.js";
import { useDrag } from "../../hooks/use_drag.js";
import { RepoWorkspacePanel } from "../repo_workspace_panel.js";
import { WorkspaceLayout } from "../layout/workspace_layout.js";
import { DragErrorNotice } from "../drag_error_notice.js";
import "../../styles/overview/overview_workspace.css";

/** A feed file opens its current content; a Git column row opens its frozen entry's diff */
export type OverviewTarget = { kind: "file"; path: string } | { kind: "diff"; entry: SourceControlEntry };

interface OverviewWorkspaceProps {
  repo: RepoConfig;
  target: OverviewTarget;
  onClose: () => void;
  isHandset: boolean;
}

export function OverviewWorkspace({
  repo,
  target,
  onClose,
  isHandset,
}: OverviewWorkspaceProps): React.JSX.Element {
  const { workspace, openFile, openDiff, closeWorkspace, updateTextScratch } = useRepoWorkspace(repo.repoId);
  const { dragState, handleDragStart, clearError } = useDrag();
  const openRef = useRef({ openFile, openDiff });

  useEffect(() => { openRef.current = { openFile, openDiff }; }, [openFile, openDiff]);
  useEffect(() => {
    if (target.kind === "file") openRef.current.openFile(target.path);
    else openRef.current.openDiff(target.entry);
  }, [repo.repoId, repo.root.kind, repo.root.path, target]);

  const handleClose = useCallback(() => {
    closeWorkspace();
    onClose();
  }, [closeWorkspace, onClose]);
  const handleFileDrag = useCallback((filePath: string) => {
    void handleDragStart(repo.repoId, filePath, undefined);
  }, [handleDragStart, repo.repoId]);

  return (
    <div className="tab overview-workspace">
      {dragState.error && <DragErrorNotice message={dragState.error} onDismiss={clearError} />}
      {workspace.kind !== "none" && workspace.kind !== "note" ? (
        <RepoWorkspacePanel
          workspace={workspace}
          repoLabel={repo.label}
          isHandset={isHandset}
          onClose={handleClose}
          onTextChange={updateTextScratch}
          onTextFileDragStart={handleFileDrag}
          onImageDragStart={handleFileDrag}
        />
      ) : (
        <WorkspaceLayout
          title={repo.label}
          subtitle={target.kind === "file" ? target.path : target.entry.path}
          onClose={handleClose}
          content={<p className="empty-state empty-state--waiting">{target.kind === "file" ? "Opening file" : "Opening diff"}</p>}
          isHandset={isHandset}
        />
      )}
    </div>
  );
}
