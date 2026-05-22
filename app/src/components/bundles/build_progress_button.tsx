// Path: app/src/components/bundles/build_progress_button.tsx
// Description: Bundle build/cancel button with inline progress details

import React from "react";
import type { BundleBuildPhase } from "../../shared/protocol.js";

interface BuildProgressValue {
  phase: BundleBuildPhase;
  filesDone: number;
  filesTotal: number;
  currentFile?: string;
  currentBytesDone?: number;
  currentBytesTotal?: number;
  bytesDoneTotalBestEffort?: number;
}

interface BuildProgressButtonProps {
  isBuilding: boolean;
  isCancelling: boolean;
  canBuild: boolean;
  buildProgress: BuildProgressValue | null;
  onBuild: () => void;
  onCancelBuild: () => void;
}

function formatPathDepth(path: string, depth: number): string {
  if (!path) return "";
  const normalized = path.replace(/\\/g, "/");
  const parts = normalized.split("/").filter(Boolean);
  if (parts.length <= depth) {
    return normalized;
  }
  return `${parts.slice(0, depth).join("/")}/…`;
}

const BuildProgressDetails = React.memo(function BuildProgressDetails({
  show,
  currentFile,
  depth,
}: {
  show: boolean;
  currentFile?: string;
  depth: number;
}): React.JSX.Element {
  const displayPath = show ? formatPathDepth(currentFile ?? "", depth) : "";
  return (
    <div
      className={`build-progress-details${show ? "" : " hidden"}`}
      aria-live={show ? "polite" : "off"}
    >
      <span className="build-progress-file" title={currentFile}>
        {show ? `Writing ${displayPath}` : " "}
      </span>
    </div>
  );
});

export function BuildProgressButton({
  isBuilding,
  isCancelling,
  canBuild,
  buildProgress,
  onBuild,
  onCancelBuild,
}: BuildProgressButtonProps): React.JSX.Element {
  const currentFile = buildProgress?.currentFile;
  const showProgressDetails = Boolean(isBuilding && currentFile);
  const buildButtonClass = [
    "build-button",
    isBuilding ? "building" : "",
    isBuilding ? "cancel" : "",
    isCancelling ? "cancelling" : "",
  ].filter(Boolean).join(" ");
  const buildButtonText = isCancelling
    ? "Cancelling..."
    : isBuilding
      ? "Cancel"
      : "Build Bundle";

  return (
    <>
      <button
        className={buildButtonClass}
        onClick={isBuilding ? onCancelBuild : onBuild}
        disabled={isCancelling || (!isBuilding && !canBuild)}
        aria-busy={isBuilding}
      >
        {buildButtonText}
        {isBuilding && buildProgress && (
          <span
            className={`build-button-progress${buildProgress.filesTotal === 0 ? " indeterminate" : ""}`}
            role="progressbar"
            aria-valuenow={buildProgress.filesTotal > 0 ? buildProgress.filesDone : undefined}
            aria-valuemax={buildProgress.filesTotal > 0 ? buildProgress.filesTotal : undefined}
            aria-label="Build progress"
            style={
              buildProgress.filesTotal > 0
                ? { width: `${Math.round((buildProgress.filesDone / buildProgress.filesTotal) * 100)}%` }
                : undefined
            }
          />
        )}
      </button>

      <BuildProgressDetails
        show={showProgressDetails}
        depth={4}
        {...(currentFile !== undefined ? { currentFile } : {})}
      />
    </>
  );
}
