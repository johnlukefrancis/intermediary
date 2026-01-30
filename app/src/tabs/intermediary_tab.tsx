// Path: app/src/tabs/intermediary_tab.tsx
// Description: Intermediary project tab with file lists

import type React from "react";
import { useCallback } from "react";
import { startDrag } from "@crabnebula/tauri-plugin-drag";
import { ThreeColumn } from "../components/layout/three_column.js";
import { FileListColumn } from "../components/file_list_column.js";
import { BundleColumn } from "../components/bundles/bundle_column.js";
import { DragErrorNotice } from "../components/drag_error_notice.js";
import { useRepoState } from "../hooks/use_repo_state.js";
import { useBundleState } from "../hooks/use_bundle_state.js";
import { useDrag } from "../hooks/use_drag.js";
import { useAgent } from "../hooks/use_agent.js";

const REPO_ID = "intermediary";

export function IntermediaryTab(): React.JSX.Element {
  const { connectionState, appPaths } = useAgent();
  const { recentDocs, recentCode, stagedByPath, isLoading, topLevelDirs, registerStaged } =
    useRepoState(REPO_ID);
  const bundleState = useBundleState(REPO_ID, topLevelDirs);
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
    <div className="tab intermediary-tab">
      {dragState.error && (
        <DragErrorNotice message={dragState.error} onDismiss={clearError} />
      )}
      <ThreeColumn
        docsContent={
          <FileListColumn
            files={recentDocs}
            stagedByPath={stagedByPath}
            repoId={REPO_ID}
            emptyMessage={emptyMessage}
            onDragStart={handleDragStart}
          />
        }
        codeContent={
          <FileListColumn
            files={recentCode}
            stagedByPath={stagedByPath}
            repoId={REPO_ID}
            emptyMessage={emptyMessage}
            onDragStart={handleDragStart}
          />
        }
        zipsContent={
          <BundleColumn
            repoId={REPO_ID}
            bundleState={bundleState}
            onDragStart={handleBundleDragStart}
            emptyMessage={!isConnected ? "Waiting for agent..." : "No bundles yet"}
          />
        }
      />
    </div>
  );
}
