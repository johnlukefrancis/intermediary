// Path: app/src/tabs/repo_tab.tsx
// Description: Generic repo tab component with conditional layout (3-column or handset)

import type React from "react";
import { useCallback, useEffect, useState } from "react";
import { startDrag } from "@crabnebula/tauri-plugin-drag";
import { ThreeColumn } from "../components/layout/three_column.js";
import { HandsetDeck } from "../components/layout/handset_deck.js";
import { TextWorkspaceLayout } from "../components/layout/text_workspace_layout.js";
import { FileListColumn } from "../components/file_list_column.js";
import { BundleColumn } from "../components/bundles/bundle_column.js";
import { TextWorkspaceEditor } from "../components/text_workspace.js";
import {
  CodeHeaderLeft,
  CodeHeaderRight,
  DocsHeaderLeft,
  DocsHeaderRight,
  type FilePaneView,
} from "../components/repo_pane_headers.js";
import { DragErrorNotice } from "../components/drag_error_notice.js";
import { useRepoState } from "../hooks/use_repo_state.js";
import { useBundleState } from "../hooks/use_bundle_state.js";
import { useDrag } from "../hooks/use_drag.js";
import { useAgent } from "../hooks/use_agent.js";
import { useStarredFiles } from "../hooks/use_starred_files.js";
import { useFileSelection } from "../hooks/use_file_selection.js";
import { useNotes } from "../hooks/use_notes.js";
import { getFileName, useRepoTextWorkspace } from "../hooks/use_repo_text_workspace.js";
import type { UiMode } from "../shared/config.js";
import type { FileEntry } from "../shared/protocol.js";

const MAX_NOTE_LENGTH = 100_000;
const MAX_SCRATCH_LENGTH = 1_000_000;

interface RepoTabProps {
  repoId: string;
  uiMode: UiMode;
}

/**
 * Build FileEntry[] from starred paths, reusing recent entries where available.
 * For paths not in the recent list, creates a placeholder with empty mtime.
 */
function buildStarredEntries(
  starredPaths: readonly string[],
  recentFiles: FileEntry[],
  kind: "docs" | "code"
): FileEntry[] {
  const recentByPath = new Map(recentFiles.map((f) => [f.path, f]));
  return starredPaths.map((path) => {
    const existing = recentByPath.get(path);
    if (existing) return existing;
    // Placeholder for files not in recent list (FileRow shows "—" for empty mtime)
    return { path, kind, changeType: "change", mtime: "" };
  });
}

export function RepoTab({ repoId, uiMode }: RepoTabProps): React.JSX.Element {
  const { connectionState, appPaths } = useAgent();
  const {
    recentDocs,
    recentCode,
    stagedByPath,
    isLoading,
    hydrationStatus,
    topLevelDirs,
    topLevelSubdirs,
    defaultExcluded,
    registerStaged,
  } = useRepoState(repoId);
  const bundleState = useBundleState(repoId, topLevelDirs, topLevelSubdirs, defaultExcluded);
  const { dragState, handleDragStart, handleMultiDragStart, clearError } = useDrag({
    onStaged: registerStaged,
  });
  const { starredDocsPaths, starredCodePaths } = useStarredFiles(repoId);
  const noteState = useNotes(repoId);
  const textWorkspace = useRepoTextWorkspace(repoId);

  // View state for docs and code panes
  const [docsView, setDocsView] = useState<FilePaneView>("recent");
  const [codeView, setCodeView] = useState<FilePaneView>("recent");

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

  const isConnected = connectionState.status === "connected";
  const recentEmptyMessage =
    !isConnected || hydrationStatus === "waiting_for_agent"
      ? "Waiting for agent..."
      : hydrationStatus === "hydrating" || hydrationStatus === "retrying" || isLoading
        ? "Loading..."
        : hydrationStatus === "error"
          ? "Unable to load files"
          : "No recent files";

  // Build file lists based on view.
  const docsFiles =
    docsView === "starred"
      ? buildStarredEntries(starredDocsPaths, recentDocs, "docs")
      : recentDocs;
  const codeFiles =
    codeView === "starred"
      ? buildStarredEntries(starredCodePaths, recentCode, "code")
      : recentCode;

  // Selection hooks — must be after file lists are computed
  const docsSelection = useFileSelection(docsFiles);
  const codeSelection = useFileSelection(codeFiles);

  // Per-pane drag handlers: multi-drag if file is in multi-selection, else single
  const handleDocsDrag = useCallback(
    (path: string) => {
      if (docsSelection.isSelected(path) && docsSelection.selectionCount > 1) {
        const files = [...docsSelection.selectedPaths].map((p) => ({
          path: p,
          stagedInfo: stagedByPath.get(p),
        }));
        void handleMultiDragStart(repoId, files);
      } else {
        docsSelection.clearSelection();
        void handleDragStart(repoId, path, stagedByPath.get(path));
      }
    },
    [repoId, docsSelection, stagedByPath, handleDragStart, handleMultiDragStart]
  );

  const handleCodeDrag = useCallback(
    (path: string) => {
      if (codeSelection.isSelected(path) && codeSelection.selectionCount > 1) {
        const files = [...codeSelection.selectedPaths].map((p) => ({
          path: p,
          stagedInfo: stagedByPath.get(p),
        }));
        void handleMultiDragStart(repoId, files);
      } else {
        codeSelection.clearSelection();
        void handleDragStart(repoId, path, stagedByPath.get(path));
      }
    },
    [repoId, codeSelection, stagedByPath, handleDragStart, handleMultiDragStart]
  );

  // View-switch handlers: clear selection when switching views
  const handleDocsViewChange = useCallback(
    (view: FilePaneView) => {
      setDocsView(view);
      docsSelection.clearSelection();
    },
    [docsSelection]
  );

  const handleCodeViewChange = useCallback(
    (view: FilePaneView) => {
      setCodeView(view);
      codeSelection.clearSelection();
    },
    [codeSelection]
  );

  // Escape to clear all selection
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent): void => {
      if (e.key === "Escape") {
        docsSelection.clearSelection();
        codeSelection.clearSelection();
      }
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => { document.removeEventListener("keydown", handleKeyDown); };
  }, [docsSelection, codeSelection]);

  // Empty messages per view
  const docsEmptyMessage =
    docsView === "starred" ? "No starred files" : recentEmptyMessage;
  const codeEmptyMessage =
    codeView === "starred" ? "No starred files" : recentEmptyMessage;

  const isHandset = uiMode === "handset";

  const docsHeaderLeft = (
    <DocsHeaderLeft onRecent={() => { handleDocsViewChange("recent"); }} />
  );
  const docsHeaderRight = (
    <DocsHeaderRight
      view={docsView}
      onViewChange={handleDocsViewChange}
      onOpenNote={textWorkspace.openNote}
    />
  );
  const codeHeaderLeft = (
    <CodeHeaderLeft
      view={codeView}
      onRecent={() => { handleCodeViewChange("recent"); }}
    />
  );
  const codeHeaderRight = (
    <CodeHeaderRight view={codeView} onViewChange={handleCodeViewChange} />
  );

  // Content blocks — shared between layouts
  const docsContent = (
    <FileListColumn
      files={docsFiles}
      repoId={repoId}
      kind="docs"
      emptyMessage={docsEmptyMessage}
      selectedPaths={docsSelection.selectedPaths}
      onSelect={docsSelection.handleSelect}
      onDragStart={handleDocsDrag}
      onOpen={textWorkspace.openFileScratch}
    />
  );
  const codeContent = (
    <FileListColumn
      files={codeFiles}
      repoId={repoId}
      kind="code"
      emptyMessage={codeEmptyMessage}
      selectedPaths={codeSelection.selectedPaths}
      onSelect={codeSelection.handleSelect}
      onDragStart={handleCodeDrag}
      onOpen={textWorkspace.openFileScratch}
    />
  );
  const zipsContent = (
    <BundleColumn
      repoId={repoId}
      bundleState={bundleState}
      onDragStart={handleBundleDragStart}
      emptyMessage={!isConnected ? "Waiting for agent..." : "No bundles yet"}
    />
  );

  const workspace = textWorkspace.workspace;
  const workspaceLayout = workspace.kind === "none" ? null : (
    <TextWorkspaceLayout
      title={workspace.kind === "note" ? "Note" : getFileName(workspace.path)}
      subtitle={workspace.kind === "note" ? "Repository notes" : workspace.path}
      onClose={textWorkspace.closeWorkspace}
      editorContent={
        workspace.kind === "note" ? (
          <TextWorkspaceEditor
            value={noteState.content}
            onChange={noteState.onChange}
            isLoading={noteState.isLoading}
            error={noteState.error}
            maxLength={MAX_NOTE_LENGTH}
            placeholder="Type notes here..."
            ariaLabel="Repository notes"
          />
        ) : (
          <TextWorkspaceEditor
            value={workspace.content}
            onChange={textWorkspace.updateFileScratch}
            maxLength={MAX_SCRATCH_LENGTH}
            placeholder="Empty file"
            ariaLabel={`Scratch text buffer for ${workspace.path}`}
          />
        )
      }
      zipsContent={zipsContent}
      isHandset={isHandset}
    />
  );

  return (
    <div className="tab repo-tab">
      {dragState.error && (
        <DragErrorNotice message={dragState.error} onDismiss={clearError} />
      )}
      {workspaceLayout ?? (isHandset ? (
        <HandsetDeck
          docsHeaderRight={docsHeaderRight}
          codeHeaderRight={codeHeaderRight}
          docsContent={docsContent}
          codeContent={codeContent}
          zipsContent={zipsContent}
        />
      ) : (
        <ThreeColumn
          docsHeaderLeft={docsHeaderLeft}
          docsHeaderRight={docsHeaderRight}
          docsContent={docsContent}
          codeHeaderLeft={codeHeaderLeft}
          codeHeaderRight={codeHeaderRight}
          codeContent={codeContent}
          zipsContent={zipsContent}
        />
      ))}
    </div>
  );
}
