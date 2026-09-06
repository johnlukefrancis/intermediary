// Path: app/src/tabs/repo_tab_file_panel.tsx
// Description: Chooses the Stream panel or the Auto files table for the repo tab's file slot

import type React from "react";
import { AutoFilesPanel } from "../components/auto_files_panel.js";
import { StreamPanel } from "../components/stream/stream_panel.js";
import { isStreamMode, type FilesMode } from "../lib/files/files_mode.js";
import type { FeedFileEntry, FileTypeFilter } from "../lib/files/file_feed.js";
import type { RepoHydrationStatus } from "../hooks/use_repo_state.js";
import type { RepoStream } from "../hooks/stream/use_repo_stream.js";

/** Everything the table's empty line is derived from; the stream states its own emptiness */
export interface FilePanelLoadState {
  isConnected: boolean;
  hydrationStatus: RepoHydrationStatus;
  isLoading: boolean;
  /** Recent files before the ZIP-selection and type filters */
  recentCount: number;
  /** True when a ZIP selection is active and hides every recent file */
  hiddenByBundleSelection: boolean;
}

interface RepoTabFilePanelProps {
  repoId: string;
  repoLabel: string;
  mode: FilesMode;
  filter: FileTypeFilter;
  /** Ranked, filtered table feed; empty in stream mode */
  feedFiles: FeedFileEntry[];
  /** The live store binding the stream renders */
  stream: RepoStream;
  handset: boolean;
  selectedPaths: ReadonlySet<string>;
  headerPrefix?: React.ReactNode;
  loadState: FilePanelLoadState;
  onFilterChange: (filter: FileTypeFilter) => void;
  onModeChange: (mode: FilesMode) => void;
  onSelect: (
    path: string,
    event: Pick<React.MouseEvent, "ctrlKey" | "metaKey" | "shiftKey">
  ) => void;
  onDragStart: (path: string) => void | Promise<void>;
  onOpen: (path: string) => void;
}

function recentEmptyMessage(load: FilePanelLoadState): string {
  if (!load.isConnected || load.hydrationStatus === "waiting_for_agent") {
    return "Waiting for agent...";
  }
  if (load.hydrationStatus === "hydrating" || load.hydrationStatus === "retrying" || load.isLoading) {
    return "Loading...";
  }
  if (load.hydrationStatus === "error") return "Unable to load files";
  return "No recent files";
}

function tableEmptyMessage(load: FilePanelLoadState, feedCount: number): string {
  if (load.recentCount > 0 && feedCount === 0) {
    return load.hiddenByBundleSelection ? "Hidden by ZIP selection" : "No matching files";
  }
  return recentEmptyMessage(load);
}

export function RepoTabFilePanel({
  repoId,
  repoLabel,
  mode,
  filter,
  feedFiles,
  stream,
  handset,
  selectedPaths,
  headerPrefix,
  loadState,
  onFilterChange,
  onModeChange,
  onSelect,
  onDragStart,
  onOpen,
}: RepoTabFilePanelProps): React.JSX.Element {
  if (isStreamMode(mode)) {
    // Keyed by repo: the panel and its tile owner (useStreamImages) never outlive a repo switch
    return (
      <StreamPanel
        key={repoId}
        repoId={repoId}
        repoLabel={repoLabel}
        filter={filter}
        mode={mode}
        handset={handset}
        headerPrefix={headerPrefix}
        stream={stream}
        onFilterChange={onFilterChange}
        onModeChange={onModeChange}
      />
    );
  }

  return (
    <AutoFilesPanel
      files={feedFiles}
      repoId={repoId}
      emptyMessage={tableEmptyMessage(loadState, feedFiles.length)}
      filter={filter}
      mode={mode}
      selectedPaths={selectedPaths}
      headerPrefix={headerPrefix}
      onFilterChange={onFilterChange}
      onModeChange={onModeChange}
      onSelect={onSelect}
      onDragStart={onDragStart}
      onOpen={onOpen}
    />
  );
}
