// Path: app/src/tabs/repo_tab.tsx
// Description: Generic repo tab component with latest and active file feeds

import type React from "react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { startDrag } from "@crabnebula/tauri-plugin-drag";
import { ThreeColumn } from "../components/layout/three_column.js";
import { HandsetDeck } from "../components/layout/handset_deck.js";
import { FileListColumn } from "../components/file_list_column.js";
import { BundleColumn } from "../components/bundles/bundle_column.js";
import { RepoWorkspacePanel } from "../components/repo_workspace_panel.js";
import { FileFeedHeader } from "../components/repo_pane_headers.js";
import { DragErrorNotice } from "../components/drag_error_notice.js";
import { useRepoState } from "../hooks/use_repo_state.js";
import { useBundleState } from "../hooks/use_bundle_state.js";
import { useDrag } from "../hooks/use_drag.js";
import { useAgent } from "../hooks/use_agent.js";
import { useFileSelection } from "../hooks/use_file_selection.js";
import { useNotes } from "../hooks/use_notes.js";
import { useRepoWorkspace } from "../hooks/use_repo_workspace.js";
import {
  filterFeedFiles,
  sortActiveFeed,
  sortLatestFeed,
  type FileTypeFilter,
} from "../lib/files/file_feed.js";
import type { UiMode } from "../shared/config.js";

interface RepoTabProps {
  repoId: string;
  uiMode: UiMode;
}

export function RepoTab({ repoId, uiMode }: RepoTabProps): React.JSX.Element {
  const { connectionState, appPaths } = useAgent();
  const {
    recentFiles,
    stagedByPath,
    isLoading,
    hydrationStatus,
    topLevelDirs,
    topLevelFiles,
    topLevelSubdirs,
    defaultExcluded,
    registerStaged,
  } = useRepoState(repoId);
  const bundleState = useBundleState(repoId, topLevelDirs, topLevelSubdirs, defaultExcluded);
  const { dragState, handleDragStart, handleMultiDragStart, clearError } = useDrag({
    onStaged: registerStaged,
  });
  const noteState = useNotes(repoId);
  const repoWorkspace = useRepoWorkspace(repoId);
  const [latestFilter, setLatestFilter] = useState<FileTypeFilter>("all");
  const [activeFilter, setActiveFilter] = useState<FileTypeFilter>("all");

  const latestFiles = useMemo(
    () => sortLatestFeed(filterFeedFiles(recentFiles, latestFilter)),
    [latestFilter, recentFiles]
  );
  const activeFiles = useMemo(
    () => sortActiveFeed(filterFeedFiles(recentFiles, activeFilter)),
    [activeFilter, recentFiles]
  );

  const latestSelection = useFileSelection(latestFiles);
  const activeSelection = useFileSelection(activeFiles);

  const handleBundleDragStart = useCallback(
    async (hostPath: string) => {
      if (!appPaths) return;
      await startDrag({
        item: [hostPath],
        icon: appPaths.dragIconHostPath,
      });
    },
    [appPaths]
  );

  const handleLatestDrag = useCallback(
    (path: string) => {
      if (latestSelection.isSelected(path) && latestSelection.selectionCount > 1) {
        const files = [...latestSelection.selectedPaths].map((p) => ({
          path: p,
          stagedInfo: stagedByPath.get(p),
        }));
        void handleMultiDragStart(repoId, files);
      } else {
        latestSelection.clearSelection();
        void handleDragStart(repoId, path, stagedByPath.get(path));
      }
    },
    [repoId, latestSelection, stagedByPath, handleDragStart, handleMultiDragStart]
  );

  const handleActiveDrag = useCallback(
    (path: string) => {
      if (activeSelection.isSelected(path) && activeSelection.selectionCount > 1) {
        const files = [...activeSelection.selectedPaths].map((p) => ({
          path: p,
          stagedInfo: stagedByPath.get(p),
        }));
        void handleMultiDragStart(repoId, files);
      } else {
        activeSelection.clearSelection();
        void handleDragStart(repoId, path, stagedByPath.get(path));
      }
    },
    [repoId, activeSelection, stagedByPath, handleDragStart, handleMultiDragStart]
  );

  const handleLatestFilterChange = useCallback(
    (filter: FileTypeFilter) => {
      setLatestFilter(filter);
      latestSelection.clearSelection();
    },
    [latestSelection]
  );

  const handleActiveFilterChange = useCallback(
    (filter: FileTypeFilter) => {
      setActiveFilter(filter);
      activeSelection.clearSelection();
    },
    [activeSelection]
  );

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent): void => {
      if (e.key === "Escape") {
        latestSelection.clearSelection();
        activeSelection.clearSelection();
      }
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => { document.removeEventListener("keydown", handleKeyDown); };
  }, [latestSelection, activeSelection]);

  const isConnected = connectionState.status === "connected";
  const recentEmptyMessage =
    !isConnected || hydrationStatus === "waiting_for_agent"
      ? "Waiting for agent..."
      : hydrationStatus === "hydrating" || hydrationStatus === "retrying" || isLoading
        ? "Loading..."
        : hydrationStatus === "error"
          ? "Unable to load files"
          : "No recent files";

  const latestEmptyMessage =
    recentFiles.length > 0 && latestFiles.length === 0 ? "No matching files" : recentEmptyMessage;
  const activeEmptyMessage =
    recentFiles.length > 0 && activeFiles.length === 0 ? "No matching files" : recentEmptyMessage;

  const isHandset = uiMode === "handset";

  const latestHeader = (
    <FileFeedHeader
      title="Latest"
      filter={latestFilter}
      onFilterChange={handleLatestFilterChange}
      onOpenNote={repoWorkspace.openNote}
      showTitle={!isHandset}
    />
  );
  const activeHeader = (
    <FileFeedHeader
      title="Active"
      filter={activeFilter}
      onFilterChange={handleActiveFilterChange}
      showTitle={!isHandset}
    />
  );

  const latestContent = (
    <FileListColumn
      files={latestFiles}
      repoId={repoId}
      emptyMessage={latestEmptyMessage}
      selectedPaths={latestSelection.selectedPaths}
      onSelect={latestSelection.handleSelect}
      onDragStart={handleLatestDrag}
      onOpen={repoWorkspace.openFile}
    />
  );
  const activeContent = (
    <FileListColumn
      files={activeFiles}
      repoId={repoId}
      emptyMessage={activeEmptyMessage}
      selectedPaths={activeSelection.selectedPaths}
      onSelect={activeSelection.handleSelect}
      onDragStart={handleActiveDrag}
      onOpen={repoWorkspace.openFile}
    />
  );
  const zipsContent = (
    <BundleColumn
      repoId={repoId}
      bundleState={bundleState}
      topLevelFiles={topLevelFiles}
      onDragStart={handleBundleDragStart}
      onOpenFile={repoWorkspace.openFile}
      emptyMessage={!isConnected ? "Waiting for agent..." : "No bundles yet"}
    />
  );

  const workspace = repoWorkspace.workspace;
  const workspaceLayout = workspace.kind === "none" ? null : (
    <RepoWorkspacePanel
      workspace={workspace}
      noteState={noteState}
      zipsContent={zipsContent}
      isHandset={isHandset}
      onClose={repoWorkspace.closeWorkspace}
      onTextChange={repoWorkspace.updateTextScratch}
      onImageDragStart={(path) => {
        void handleDragStart(repoId, path, stagedByPath.get(path));
      }}
    />
  );

  return (
    <div className="tab repo-tab">
      {dragState.error && (
        <DragErrorNotice message={dragState.error} onDismiss={clearError} />
      )}
      {workspaceLayout ?? (isHandset ? (
        <HandsetDeck
          latestHeader={latestHeader}
          activeHeader={activeHeader}
          latestContent={latestContent}
          activeContent={activeContent}
          zipsContent={zipsContent}
        />
      ) : (
        <ThreeColumn
          latestHeaderLeft={latestHeader}
          latestContent={latestContent}
          activeHeaderLeft={activeHeader}
          activeContent={activeContent}
          zipsContent={zipsContent}
        />
      ))}
    </div>
  );
}
