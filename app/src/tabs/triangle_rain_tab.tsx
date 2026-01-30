// Path: app/src/tabs/triangle_rain_tab.tsx
// Description: Triangle Rain project tab with worktree selector and file lists

import React, { useState, useMemo } from "react";
import { ThreeColumn } from "../components/layout/three_column.js";
import { FileListColumn } from "../components/file_list_column.js";
import { ZipColumnPlaceholder } from "../components/zip_column_placeholder.js";
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
  const { recentDocs, recentCode, stagedByPath, isLoading } = useRepoState(repoId);
  const { handleDragStart } = useDrag();

  const isConnected = connectionState.status === "connected";
  const emptyMessage = !isConnected
    ? "Waiting for agent..."
    : isLoading
      ? "Loading..."
      : "No recent files";

  return (
    <div className="tab triangle-rain-tab">
      <div className="worktree-selector">
        <label htmlFor="worktree-select">Worktree:</label>
        <select
          id="worktree-select"
          value={selectedWorktree}
          onChange={(e) => {
            setSelectedWorktree(e.target.value as WorktreeId);
          }}
        >
          {WORKTREES.map((wt) => (
            <option key={wt.id} value={wt.id}>
              {wt.label}
            </option>
          ))}
        </select>
      </div>
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
