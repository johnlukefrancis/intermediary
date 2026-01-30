// Path: app/src/tabs/triangle_rain_tab.tsx
// Description: Triangle Rain project tab with worktree selector and file lists

import React, { useState, useMemo } from "react";
import { ThreeColumn } from "../components/layout/three_column.js";
import { FileListColumn } from "../components/file_list_column.js";
import { ZipColumnPlaceholder } from "../components/zip_column_placeholder.js";
import { DragErrorNotice } from "../components/drag_error_notice.js";
import { WorktreeSelector } from "../components/worktree_selector.js";
import { useRepoState } from "../hooks/use_repo_state.js";
import { useDrag } from "../hooks/use_drag.js";
import { useAgent } from "../hooks/use_agent.js";
import type { WorktreeId } from "../shared/protocol.js";

const WORKTREES: { id: WorktreeId; label: string }[] = [
  { id: "tr-engine", label: "tr-engine" },
];

/** Derive repoId from worktree selection */
function getRepoId(worktreeId: WorktreeId): string {
  return `triangle-rain-${worktreeId}`;
}

export function TriangleRainTab(): React.JSX.Element {
  const [selectedWorktree, setSelectedWorktree] = useState<WorktreeId>("tr-engine");
  const repoId = useMemo(() => getRepoId(selectedWorktree), [selectedWorktree]);

  const { connectionState } = useAgent();
  const { recentDocs, recentCode, stagedByPath, isLoading, registerStaged } = useRepoState(repoId);
  const { dragState, handleDragStart, clearError } = useDrag({
    onStaged: registerStaged,
  });

  const isConnected = connectionState.status === "connected";
  const emptyMessage = !isConnected
    ? "Waiting for agent..."
    : isLoading
      ? "Loading..."
      : "No recent files";

  return (
    <div className="tab triangle-rain-tab">
      <WorktreeSelector
        value={selectedWorktree}
        options={WORKTREES}
        onChange={setSelectedWorktree}
      />
      {dragState.error && (
        <DragErrorNotice message={dragState.error} onDismiss={clearError} />
      )}
      <ThreeColumn
        docsContent={
          <FileListColumn
            files={recentDocs}
            stagedByPath={stagedByPath}
            repoId={repoId}
            emptyMessage={emptyMessage}
            onDragStart={handleDragStart}
          />
        }
        codeContent={
          <FileListColumn
            files={recentCode}
            stagedByPath={stagedByPath}
            repoId={repoId}
            emptyMessage={emptyMessage}
            onDragStart={handleDragStart}
          />
        }
        zipsContent={<ZipColumnPlaceholder />}
      />
    </div>
  );
}
