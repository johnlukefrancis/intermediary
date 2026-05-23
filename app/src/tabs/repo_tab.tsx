// Path: app/src/tabs/repo_tab.tsx
// Description: Generic repo tab component with File Intelligence and zip bundles

import type React from "react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { startDrag } from "@crabnebula/tauri-plugin-drag";
import { ThreeColumn } from "../components/layout/three_column.js";
import { HandsetDeck } from "../components/layout/handset_deck.js";
import { FileIntelligencePanel } from "../components/file_intelligence_panel.js";
import { BundleColumn } from "../components/bundles/bundle_column.js";
import { RepoWorkspacePanel } from "../components/repo_workspace_panel.js";
import { DragErrorNotice } from "../components/drag_error_notice.js";
import { useRepoState } from "../hooks/use_repo_state.js";
import { useBundleState } from "../hooks/use_bundle_state.js";
import { useDrag } from "../hooks/use_drag.js";
import { useAgent } from "../hooks/use_agent.js";
import { useFileSelection } from "../hooks/use_file_selection.js";
import { useNotes } from "../hooks/use_notes.js";
import { useRepoWorkspace } from "../hooks/use_repo_workspace.js";
import {
  buildFileIntelligenceFeed,
  type FileSortMode,
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
  const [fileFilter, setFileFilter] = useState<FileTypeFilter>("all");
  const [sortMode, setSortMode] = useState<FileSortMode>("intelligence");

  const intelligenceFiles = useMemo(
    () => buildFileIntelligenceFeed(recentFiles, fileFilter, sortMode),
    [fileFilter, recentFiles, sortMode]
  );

  const fileSelection = useFileSelection(intelligenceFiles);

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

  const handleFileDrag = useCallback(
    (path: string) => {
      if (fileSelection.isSelected(path) && fileSelection.selectionCount > 1) {
        const files = [...fileSelection.selectedPaths].map((p) => ({
          path: p,
          stagedInfo: stagedByPath.get(p),
        }));
        void handleMultiDragStart(repoId, files);
      } else {
        fileSelection.clearSelection();
        void handleDragStart(repoId, path, stagedByPath.get(path));
      }
    },
    [repoId, fileSelection, stagedByPath, handleDragStart, handleMultiDragStart]
  );

  const handleFilterChange = useCallback(
    (filter: FileTypeFilter) => {
      setFileFilter(filter);
      fileSelection.clearSelection();
    },
    [fileSelection]
  );

  const handleSortModeChange = useCallback(
    (mode: FileSortMode) => {
      setSortMode(mode);
      fileSelection.clearSelection();
    },
    [fileSelection]
  );

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent): void => {
      if (e.key === "Escape") {
        fileSelection.clearSelection();
      }
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => { document.removeEventListener("keydown", handleKeyDown); };
  }, [fileSelection]);

  const isConnected = connectionState.status === "connected";
  const recentEmptyMessage =
    !isConnected || hydrationStatus === "waiting_for_agent"
      ? "Waiting for agent..."
      : hydrationStatus === "hydrating" || hydrationStatus === "retrying" || isLoading
        ? "Loading..."
        : hydrationStatus === "error"
          ? "Unable to load files"
          : "No recent files";

  const fileEmptyMessage =
    recentFiles.length > 0 && intelligenceFiles.length === 0
      ? "No matching files"
      : recentEmptyMessage;

  const isHandset = uiMode === "handset";

  const renderFilePanel = (headerPrefix?: React.ReactNode): React.JSX.Element => (
    <FileIntelligencePanel
      files={intelligenceFiles}
      repoId={repoId}
      emptyMessage={fileEmptyMessage}
      filter={fileFilter}
      sortMode={sortMode}
      selectedPaths={fileSelection.selectedPaths}
      headerPrefix={headerPrefix}
      onFilterChange={handleFilterChange}
      onSortModeChange={handleSortModeChange}
      onSelect={fileSelection.handleSelect}
      onDragStart={handleFileDrag}
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
          filePanel={renderFilePanel}
          zipsContent={zipsContent}
        />
      ) : (
        <ThreeColumn
          fileContent={renderFilePanel()}
          zipsContent={zipsContent}
        />
      ))}
    </div>
  );
}
