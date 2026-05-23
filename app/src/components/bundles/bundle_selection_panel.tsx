// Path: app/src/components/bundles/bundle_selection_panel.tsx
// Description: Bundle build controls and file explorer selection panel

import type React from "react";
import type { BundleBuildPhase, BundleSelection } from "../../shared/protocol.js";
import { BuildProgressButton } from "./build_progress_button.js";
import { BundleFileExplorer } from "./bundle_file_explorer.js";

interface BundleSelectionPanelProps {
  repoId: string;
  selection: BundleSelection;
  topLevelDirs: string[];
  topLevelFiles: string[];
  isBuilding: boolean;
  isCancelling: boolean;
  buildProgress: {
    phase: BundleBuildPhase;
    filesDone: number;
    filesTotal: number;
    currentFile?: string;
    currentBytesDone?: number;
    currentBytesTotal?: number;
    bytesDoneTotalBestEffort?: number;
  } | null;
  lastBuildError: string | null;
  onSelectionChange: (selection: BundleSelection) => void;
  onBuild: () => void;
  onCancelBuild: () => void;
  onOpenFile: (path: string) => void;
}

export function BundleSelectionPanel({
  repoId,
  selection,
  topLevelDirs,
  topLevelFiles,
  isBuilding,
  isCancelling,
  buildProgress,
  lastBuildError,
  onSelectionChange,
  onBuild,
  onCancelBuild,
  onOpenFile,
}: BundleSelectionPanelProps): React.JSX.Element {
  const canBuild = selection.includeRoot || selection.topLevelDirs.length > 0;

  return (
    <div className="bundle-selection-panel">
      <BuildProgressButton
        isBuilding={isBuilding}
        isCancelling={isCancelling}
        canBuild={canBuild}
        buildProgress={buildProgress}
        onBuild={onBuild}
        onCancelBuild={onCancelBuild}
      />

      <BundleFileExplorer
        repoId={repoId}
        selection={selection}
        topLevelDirs={topLevelDirs}
        topLevelFiles={topLevelFiles}
        onSelectionChange={onSelectionChange}
        onOpenFile={onOpenFile}
      />

      {lastBuildError && (
        <div className="build-error">{lastBuildError}</div>
      )}
    </div>
  );
}
