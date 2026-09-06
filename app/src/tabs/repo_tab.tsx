// Path: app/src/tabs/repo_tab.tsx
// Description: Generic repo tab component with Auto files, the right rail (zips | source | terminal), and the workspace

import type React from "react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { startDrag } from "@crabnebula/tauri-plugin-drag";
import { ThreeColumn } from "../components/layout/three_column.js";
import { HandsetDeck } from "../components/layout/handset_deck.js";
import { RepoRail } from "../components/layout/repo_rail.js";
import { RepoWorkspacePanel } from "../components/repo_workspace_panel.js";
import { DragErrorNotice } from "../components/drag_error_notice.js";
import { useRepoState } from "../hooks/use_repo_state.js";
import { useBundleState } from "../hooks/use_bundle_state.js";
import { useConfig } from "../hooks/use_config.js";
import { useDeckSection } from "../hooks/use_deck_section.js";
import { useDrag } from "../hooks/use_drag.js";
import { useAgent } from "../hooks/use_agent.js";
import { useFileSelection } from "../hooks/use_file_selection.js";
import { useNotes } from "../hooks/use_notes.js";
import { useRepoWorkspace } from "../hooks/use_repo_workspace.js";
import { useRepoStream } from "../hooks/stream/use_repo_stream.js";
import { useSourceControlState } from "../hooks/source_control/use_source_control_state.js";
import { buildRepoRailBodies } from "./repo_tab_rail.js";
import { RepoTabFilePanel } from "./repo_tab_file_panel.js";
import { buildAutoFileFeed, type FeedFileEntry, type FileTypeFilter } from "../lib/files/file_feed.js";
import { isStreamMode, sortModeOf, type FilesMode } from "../lib/files/files_mode.js";
import { isFileIncluded } from "../lib/bundles/bundle_selection_visibility.js";
import type { UiMode } from "../shared/config.js";

interface RepoTabProps {
  repoId: string;
  uiMode: UiMode;
}

export function RepoTab({ repoId, uiMode }: RepoTabProps): React.JSX.Element {
  const { connectionState, appPaths } = useAgent();
  const { config, setRailWidthPercent, setFilesMode } = useConfig();
  const repo = config.repos.find((entry) => entry.repoId === repoId);
  const repoRoot = repo?.root;
  const {
    recentFiles,
    stagedByPath,
    isLoading,
    hydrationStatus,
    topLevelDirs,
    topLevelFiles,
    topLevelSubdirs,
    isTopologyReady,
    defaultExcluded,
    registerStaged,
  } = useRepoState(repoId);
  const bundleState = useBundleState(
    repoId,
    topLevelDirs,
    topLevelSubdirs,
    defaultExcluded,
    isTopologyReady
  );
  const { dragState, handleDragStart, handleMultiDragStart, clearError } = useDrag({
    onStaged: registerStaged,
  });
  const noteState = useNotes(repoId);
  const repoWorkspace = useRepoWorkspace(repoId);
  // Status is fetched for the active repo regardless of the active rail so the SOURCE count stays live.
  const sourceControl = useSourceControlState(repoId);
  const deckSection = useDeckSection();
  const [fileFilter, setFileFilter] = useState<FileTypeFilter>("all");
  // The panel mode is global and persisted; the type filter stays per-tab and per-session
  const filesMode = config.uiState.filesMode;
  const activePreset = bundleState.presets.get(bundleState.activePresetId);
  const activeBundleSelection =
    activePreset?.isSelectionInitialized && activePreset.isSelectionTopologyReady ? activePreset.selection : null;

  const bundleVisibleRecentFiles = useMemo(
    () => activeBundleSelection
      ? recentFiles.filter((file) => isFileIncluded(file.path, activeBundleSelection))
      : recentFiles,
    [activeBundleSelection, recentFiles]
  );

  // The table feed is only built for the table modes; the stream seeds from the recent list itself
  const feedFiles = useMemo<FeedFileEntry[]>(
    () => isStreamMode(filesMode)
      ? []
      : buildAutoFileFeed(bundleVisibleRecentFiles, fileFilter, sortModeOf(filesMode, "auto")),
    [bundleVisibleRecentFiles, fileFilter, filesMode]
  );

  const fileSelection = useFileSelection(feedFiles);

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

  const handleModeChange = useCallback(
    (mode: FilesMode) => {
      setFilesMode(mode);
      fileSelection.clearSelection();
    },
    [fileSelection, setFilesMode]
  );

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent): void => {
      // Escape inside the terminal belongs to the shell (vim, a TUI), not to the file selection
      const inTerminal =
        e.target instanceof Element && e.target.closest("[data-terminal-host]") !== null;
      if (e.key === "Escape" && !inTerminal) {
        fileSelection.clearSelection();
      }
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => { document.removeEventListener("keydown", handleKeyDown); };
  }, [fileSelection]);

  const isConnected = connectionState.status === "connected";
  const isHandset = uiMode === "handset";
  // The store keeps admitting either way; visible only drives cadence and the while-away count
  const streamVisible = isStreamMode(filesMode) && repoWorkspace.workspace.kind === "none"
    && (!isHandset || deckSection.handsetSection === "files");
  const stream = useRepoStream(repoId, {
    visible: streamVisible, bundleSelection: activeBundleSelection,
    openFile: repoWorkspace.openFile, openDiff: repoWorkspace.openDiff, onDragStart: handleFileDrag,
  });

  const renderFilePanel = (headerPrefix?: React.ReactNode): React.JSX.Element => (
    <RepoTabFilePanel
      repoId={repoId}
      repoLabel={repo?.label ?? repoId}
      mode={filesMode}
      filter={fileFilter}
      feedFiles={feedFiles}
      stream={stream}
      handset={isHandset}
      selectedPaths={fileSelection.selectedPaths}
      headerPrefix={headerPrefix}
      loadState={{
        isConnected,
        hydrationStatus,
        isLoading,
        recentCount: recentFiles.length,
        hiddenByBundleSelection:
          activeBundleSelection !== null && bundleVisibleRecentFiles.length === 0,
      }}
      onFilterChange={handleFilterChange}
      onModeChange={handleModeChange}
      onSelect={fileSelection.handleSelect}
      onDragStart={handleFileDrag}
      onOpen={repoWorkspace.openFile}
    />
  );
  const railBodies = buildRepoRailBodies({
    repoId,
    repoRoot,
    isConnected,
    bundleState,
    topLevelFiles,
    sourceControl,
    onBundleDragStart: handleBundleDragStart,
    onOpenFile: repoWorkspace.openFile,
    onOpenDiff: repoWorkspace.openDiff,
  });
  const railContent = (
    <RepoRail
      activeRail={deckSection.activeRail}
      sourceCount={sourceControl.changeCount}
      sourceConflictCount={sourceControl.conflictCount}
      onChangeRail={deckSection.setActiveRail}
      bodies={railBodies}
    />
  );

  const workspace = repoWorkspace.workspace;
  const workspacePanel = workspace.kind === "none" ? null : (
    <RepoWorkspacePanel
      workspace={workspace}
      noteState={noteState}
      isHandset={isHandset}
      onClose={repoWorkspace.closeWorkspace}
      onTextChange={repoWorkspace.updateTextScratch}
      onTextFileDragStart={(path) => {
        void handleDragStart(repoId, path, stagedByPath.get(path));
      }}
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
      {isHandset ? (
        workspacePanel ?? (
          <HandsetDeck
            active={deckSection.handsetSection}
            sourceCount={sourceControl.changeCount}
            sourceConflictCount={sourceControl.conflictCount}
            onChange={deckSection.setHandsetSection}
            filePanel={renderFilePanel}
            bodies={railBodies}
          />
        )
      ) : (
        <ThreeColumn
          variant={workspacePanel === null ? "files" : "workspace"}
          railWidthPercent={config.uiState.railWidthPercent}
          onRailWidthChange={setRailWidthPercent}
          fileContent={workspacePanel ?? renderFilePanel()}
          railContent={railContent}
        />
      )}
    </div>
  );
}
