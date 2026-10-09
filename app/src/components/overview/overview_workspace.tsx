// Path: app/src/components/overview/overview_workspace.tsx
// Description: Overview's full-pane file workspace using the shared repository viewers and actions

import type React from "react";
import { useCallback, useEffect, useRef } from "react";
import type { RepoConfig } from "../../shared/config.js";
import { useRepoWorkspace } from "../../hooks/use_repo_workspace.js";
import { useDrag } from "../../hooks/use_drag.js";
import { RepoWorkspacePanel } from "../repo_workspace_panel.js";
import { WorkspaceLayout } from "../layout/workspace_layout.js";
import { DragErrorNotice } from "../drag_error_notice.js";
import "../../styles/overview/overview_workspace.css";

interface OverviewWorkspaceProps {
  repo: RepoConfig;
  path: string;
  onClose: () => void;
  isHandset: boolean;
}

export function OverviewWorkspace({
  repo,
  path,
  onClose,
  isHandset,
}: OverviewWorkspaceProps): React.JSX.Element {
  const { workspace, openFile, closeWorkspace, updateTextScratch } = useRepoWorkspace(repo.repoId);
  const { dragState, handleDragStart, clearError } = useDrag();
  const openFileRef = useRef(openFile);

  useEffect(() => { openFileRef.current = openFile; }, [openFile]);
  useEffect(() => {
    openFileRef.current(path);
  }, [repo.repoId, repo.root.kind, repo.root.path, path]);

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
      {workspace.kind === "textFile" || workspace.kind === "imageFile" ? (
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
          subtitle={path}
          onClose={handleClose}
          content={<p className="empty-state empty-state--waiting">Opening file</p>}
          isHandset={isHandset}
        />
      )}
    </div>
  );
}
