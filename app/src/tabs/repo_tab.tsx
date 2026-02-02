// Path: app/src/tabs/repo_tab.tsx
// Description: Generic repo tab component with 3-column layout

import type React from "react";
import { useCallback, useState } from "react";
import { startDrag } from "@crabnebula/tauri-plugin-drag";
import { ThreeColumn } from "../components/layout/three_column.js";
import { FileListColumn } from "../components/file_list_column.js";
import { BundleColumn } from "../components/bundles/bundle_column.js";
import { DragErrorNotice } from "../components/drag_error_notice.js";
import { useRepoState } from "../hooks/use_repo_state.js";
import { useBundleState } from "../hooks/use_bundle_state.js";
import { useDrag } from "../hooks/use_drag.js";
import { useAgent } from "../hooks/use_agent.js";
import { useStarredFiles } from "../hooks/use_starred_files.js";
import type { FileEntry } from "../shared/protocol.js";

type PaneView = "recent" | "starred";

interface RepoTabProps {
  repoId: string;
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

export function RepoTab({ repoId }: RepoTabProps): React.JSX.Element {
  const { connectionState, appPaths } = useAgent();
  const {
    recentDocs,
    recentCode,
    stagedByPath,
    isLoading,
    topLevelDirs,
    topLevelSubdirs,
    registerStaged,
  } = useRepoState(repoId);
  const bundleState = useBundleState(repoId, topLevelDirs, topLevelSubdirs);
  const { dragState, handleDragStart, clearError } = useDrag({
    onStaged: registerStaged,
  });
  const { starredDocsPaths, starredCodePaths } = useStarredFiles(repoId);

  // View state for docs and code panes
  const [docsView, setDocsView] = useState<PaneView>("recent");
  const [codeView, setCodeView] = useState<PaneView>("recent");

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
  const recentEmptyMessage = !isConnected
    ? "Waiting for agent..."
    : isLoading
      ? "Loading..."
      : "No recent files";

  // Build file lists based on view
  const docsFiles =
    docsView === "starred"
      ? buildStarredEntries(starredDocsPaths, recentDocs, "docs")
      : recentDocs;
  const codeFiles =
    codeView === "starred"
      ? buildStarredEntries(starredCodePaths, recentCode, "code")
      : recentCode;

  // Empty messages per view
  const docsEmptyMessage =
    docsView === "starred" ? "No starred files" : recentEmptyMessage;
  const codeEmptyMessage =
    codeView === "starred" ? "No starred files" : recentEmptyMessage;

  // Header components for docs pane
  const docsHeaderLeft = (
    <button
      type="button"
      className="panel-title-button"
      onClick={() => { setDocsView("recent"); }}
      title="Show recent files"
    >
      Docs
    </button>
  );
  const docsHeaderRight = (
    <button
      type="button"
      className={`panel-header-icon${docsView === "starred" ? " panel-header-icon--active" : ""}`}
      onClick={() => { setDocsView(docsView === "starred" ? "recent" : "starred"); }}
      title={docsView === "starred" ? "Show recent docs" : "Show favourited docs"}
      aria-label={docsView === "starred" ? "Show recent docs" : "Show favourited docs"}
      aria-pressed={docsView === "starred"}
    >
      ★
    </button>
  );

  // Header components for code pane
  const codeHeaderLeft = (
    <button
      type="button"
      className="panel-title-button"
      onClick={() => { setCodeView("recent"); }}
      title="Show recent files"
    >
      Code
    </button>
  );
  const codeHeaderRight = (
    <button
      type="button"
      className={`panel-header-icon${codeView === "starred" ? " panel-header-icon--active" : ""}`}
      onClick={() => { setCodeView(codeView === "starred" ? "recent" : "starred"); }}
      title={codeView === "starred" ? "Show recent files" : "Show favourited files"}
      aria-label={codeView === "starred" ? "Show recent files" : "Show favourited files"}
      aria-pressed={codeView === "starred"}
    >
      ★
    </button>
  );

  return (
    <div className="tab repo-tab">
      {dragState.error && (
        <DragErrorNotice message={dragState.error} onDismiss={clearError} />
      )}
      <ThreeColumn
        docsHeaderLeft={docsHeaderLeft}
        docsHeaderRight={docsHeaderRight}
        docsContent={
          <FileListColumn
            files={docsFiles}
            stagedByPath={stagedByPath}
            repoId={repoId}
            kind="docs"
            emptyMessage={docsEmptyMessage}
            onDragStart={handleDragStart}
          />
        }
        codeHeaderLeft={codeHeaderLeft}
        codeHeaderRight={codeHeaderRight}
        codeContent={
          <FileListColumn
            files={codeFiles}
            stagedByPath={stagedByPath}
            repoId={repoId}
            kind="code"
            emptyMessage={codeEmptyMessage}
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
