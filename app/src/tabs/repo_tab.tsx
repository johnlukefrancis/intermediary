// Path: app/src/tabs/repo_tab.tsx
// Description: Generic repo tab component with conditional layout (3-column or handset)

import type React from "react";
import { useCallback, useEffect, useState } from "react";
import { startDrag } from "@crabnebula/tauri-plugin-drag";
import { ThreeColumn } from "../components/layout/three_column.js";
import { HandsetDeck } from "../components/layout/handset_deck.js";
import { FileListColumn } from "../components/file_list_column.js";
import { BundleColumn } from "../components/bundles/bundle_column.js";
import { NotePanel } from "../components/note_panel.js";
import { DragErrorNotice } from "../components/drag_error_notice.js";
import { useRepoState } from "../hooks/use_repo_state.js";
import { useBundleState } from "../hooks/use_bundle_state.js";
import { useDrag } from "../hooks/use_drag.js";
import { useAgent } from "../hooks/use_agent.js";
import { useStarredFiles } from "../hooks/use_starred_files.js";
import { useFileSelection } from "../hooks/use_file_selection.js";
import { useNotes } from "../hooks/use_notes.js";
import type { UiMode } from "../shared/config.js";
import type { FileEntry } from "../shared/protocol.js";

type PaneView = "recent" | "starred" | "notes";

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

  // View state for docs and code panes
  const [docsView, setDocsView] = useState<PaneView>("recent");
  const [codeView, setCodeView] = useState<PaneView>("recent");

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

  // Build file lists based on view (only needed when not in notes view)
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
    (view: PaneView) => {
      setDocsView(view);
      docsSelection.clearSelection();
    },
    [docsSelection]
  );

  const handleCodeViewChange = useCallback(
    (view: PaneView) => {
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

  // Track which docs sub-view was active before switching to notes
  // so clicking DOCS returns to recent (not starred)
  const isDocsFileView = docsView === "recent" || docsView === "starred";

  // Header left: DOCS | NOTES toggle
  const docsHeaderLeft = (
    <div className="docs-notes-toggle">
      <button
        type="button"
        className={`docs-notes-toggle__btn${isDocsFileView ? " docs-notes-toggle__btn--active" : ""}`}
        onClick={() => { handleDocsViewChange("recent"); }}
        title="Show docs"
      >
        Docs
      </button>
      <span className="docs-notes-toggle__sep" aria-hidden="true">|</span>
      <button
        type="button"
        className={`docs-notes-toggle__btn${docsView === "notes" ? " docs-notes-toggle__btn--active" : ""}`}
        onClick={() => { handleDocsViewChange("notes"); }}
        title="Show notes"
      >
        Notes
      </button>
    </div>
  );

  // Header right: optional handset notes toggle + star toggle
  const docsHeaderRight = (
    <div className="panel-header-icons">
      {isHandset && (
        <button
          type="button"
          className={`panel-header-icon${docsView === "notes" ? " panel-header-icon--active" : ""}`}
          onClick={() => { handleDocsViewChange(docsView === "notes" ? "recent" : "notes"); }}
          title={docsView === "notes" ? "Show docs" : "Show notes"}
          aria-label={docsView === "notes" ? "Show docs" : "Show notes"}
          aria-pressed={docsView === "notes"}
        >
          ✎
        </button>
      )}
      {docsView !== "notes" && (
        <button
          type="button"
          className={`panel-header-icon${docsView === "starred" ? " panel-header-icon--active" : ""}`}
          onClick={() => { handleDocsViewChange(docsView === "starred" ? "recent" : "starred"); }}
          title={docsView === "starred" ? "Show recent docs" : "Show favourited docs"}
          aria-label={docsView === "starred" ? "Show recent docs" : "Show favourited docs"}
          aria-pressed={docsView === "starred"}
        >
          ★
        </button>
      )}
    </div>
  );

  // Header components for code pane
  const codeHeaderLeft = (
    <button
      type="button"
      className={`panel-title-button${codeView === "starred" ? " panel-title-button--dimmed" : ""}`}
      onClick={() => { handleCodeViewChange("recent"); }}
      title="Show recent files"
    >
      Code
    </button>
  );
  const codeHeaderRight = (
    <button
      type="button"
      className={`panel-header-icon${codeView === "starred" ? " panel-header-icon--active" : ""}`}
      onClick={() => { handleCodeViewChange(codeView === "starred" ? "recent" : "starred"); }}
      title={codeView === "starred" ? "Show recent files" : "Show favourited files"}
      aria-label={codeView === "starred" ? "Show recent files" : "Show favourited files"}
      aria-pressed={codeView === "starred"}
    >
      ★
    </button>
  );

  // Content blocks — shared between layouts
  const docsContent = docsView === "notes"
    ? <NotePanel noteState={noteState} />
    : (
      <FileListColumn
        files={docsFiles}
        repoId={repoId}
        kind="docs"
        emptyMessage={docsEmptyMessage}
        selectedPaths={docsSelection.selectedPaths}
        onSelect={docsSelection.handleSelect}
        onDragStart={handleDocsDrag}
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

  return (
    <div className="tab repo-tab">
      {dragState.error && (
        <DragErrorNotice message={dragState.error} onDismiss={clearError} />
      )}
      {isHandset ? (
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
      )}
    </div>
  );
}
