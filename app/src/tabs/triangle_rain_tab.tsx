// Path: app/src/tabs/triangle_rain_tab.tsx
// Description: Triangle Rain project tab with file lists (worktree selector in tab bar)

import React, { useMemo, useCallback } from "react";
import { startDrag } from "@crabnebula/tauri-plugin-drag";
import { ThreeColumn } from "../components/layout/three_column.js";
import { FileListColumn } from "../components/file_list_column.js";
import { BundleColumn } from "../components/bundles/bundle_column.js";
import { DragErrorNotice } from "../components/drag_error_notice.js";
import { useRepoState } from "../hooks/use_repo_state.js";
import { useBundleState } from "../hooks/use_bundle_state.js";
import { useDrag } from "../hooks/use_drag.js";
import { useAgent } from "../hooks/use_agent.js";
import { useConfig } from "../hooks/use_config.js";
import type { WorktreeId } from "../shared/ids.js";

/** Derive repoId from worktree selection */
function getRepoId(worktreeId: WorktreeId): string {
  return `triangle-rain-${worktreeId}`;
}

export function TriangleRainTab(): React.JSX.Element {
  const { config } = useConfig();

  // Read worktree selection from config (managed by TabBar dropdown)
  const selectedWorktree = config.uiState.lastTriangleRainWorktreeId ?? "tr-engine";
  const repoId = useMemo(() => getRepoId(selectedWorktree), [selectedWorktree]);

  const { connectionState, appPaths } = useAgent();
  const { recentDocs, recentCode, stagedByPath, isLoading, topLevelDirs, registerStaged } =
    useRepoState(repoId);
  const bundleState = useBundleState(repoId, topLevelDirs);
  const { dragState, handleDragStart, clearError } = useDrag({
    onStaged: registerStaged,
  });

  const handleBundleDragStart = useCallback(
    async (windowsPath: string) => {
      if (!appPaths) return;
      await startDrag({
        item: [windowsPath],
        icon: appPaths.dragIconWindowsPath,
      });
    },
    [appPaths]
  );

  const isConnected = connectionState.status === "connected";
  const emptyMessage = !isConnected
    ? "Waiting for agent..."
    : isLoading
      ? "Loading..."
      : "No recent files";

  return (
    <div className="tab triangle-rain-tab">
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
        zipsContent={
          <BundleColumn
            repoId={repoId}
            bundleState={bundleState}
            onDragStart={handleBundleDragStart}
            emptyMessage={!isConnected ? "Waiting for agent..." : "No bundles yet"}
          />
        }
      />
    </div>
  );
}
