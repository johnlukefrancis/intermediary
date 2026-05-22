// Path: app/src/components/bundles/bundle_selection_panel.tsx
// Description: Selection UI for bundle building (root toggle, dir checkboxes, subdir exclusions)

import React from "react";
import { useCallback, useState } from "react";
import type { BundleBuildPhase, BundleSelection } from "../../shared/protocol.js";
import { BuildProgressButton } from "./build_progress_button.js";
import { BundleSubdirTree } from "./bundle_subdir_tree.js";
import { IndeterminateCheckbox } from "./indeterminate_checkbox.js";

interface BundleSelectionPanelProps {
  selection: BundleSelection;
  topLevelDirs: string[];
  topLevelSubdirs: Record<string, string[]>;
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
}

export function BundleSelectionPanel({
  selection,
  topLevelDirs,
  topLevelSubdirs,
  isBuilding,
  isCancelling,
  buildProgress,
  lastBuildError,
  onSelectionChange,
  onBuild,
  onCancelBuild,
}: BundleSelectionPanelProps): React.JSX.Element {
  const [expandedDirs, setExpandedDirs] = useState<Set<string>>(() => new Set());
  const selectedDirs = new Set(selection.topLevelDirs);
  const excludedSubdirs = new Set(selection.excludedSubdirs);
  const allSelected =
    topLevelDirs.length > 0 && selection.topLevelDirs.length === topLevelDirs.length;
  const noneSelected = selection.topLevelDirs.length === 0;
  const canBuild = selection.includeRoot || selection.topLevelDirs.length > 0;

  const handleIncludeRootChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      onSelectionChange({ ...selection, includeRoot: e.target.checked });
    },
    [selection, onSelectionChange]
  );

  const handleDirToggle = useCallback(
    (dir: string) => {
      const currentSet = new Set(selection.topLevelDirs);
      const excludedSet = new Set(selection.excludedSubdirs);

      if (currentSet.has(dir)) {
        currentSet.delete(dir);
        for (const path of excludedSet) {
          if (path === dir || path.startsWith(`${dir}/`)) {
            excludedSet.delete(path);
          }
        }
      } else {
        currentSet.add(dir);
      }

      onSelectionChange({
        ...selection,
        topLevelDirs: Array.from(currentSet).sort(),
        excludedSubdirs: Array.from(excludedSet).sort(),
      });
    },
    [selection, onSelectionChange]
  );

  const handleSelectAll = useCallback(() => {
    onSelectionChange({ ...selection, topLevelDirs: [...topLevelDirs].sort() });
  }, [selection, topLevelDirs, onSelectionChange]);

  const handleSelectNone = useCallback(() => {
    onSelectionChange({ ...selection, topLevelDirs: [] });
  }, [selection, onSelectionChange]);

  const handleToggleExpand = useCallback((dir: string) => {
    setExpandedDirs((prev) => {
      const next = new Set(prev);
      if (next.has(dir)) {
        next.delete(dir);
      } else {
        next.add(dir);
      }
      return next;
    });
  }, []);

  const handleSubdirToggle = useCallback(
    (subdirPath: string) => {
      const currentExcluded = new Set(selection.excludedSubdirs);

      if (currentExcluded.has(subdirPath)) {
        currentExcluded.delete(subdirPath);
      } else {
        currentExcluded.add(subdirPath);
      }

      onSelectionChange({
        ...selection,
        excludedSubdirs: Array.from(currentExcluded).sort(),
      });
    },
    [selection, onSelectionChange]
  );

  /** Check if a directory has any excluded subdirs */
  const hasExcludedSubdirs = useCallback(
    (dir: string): boolean => {
      return selection.excludedSubdirs.some((path) => path.startsWith(`${dir}/`));
    },
    [selection.excludedSubdirs]
  );

  /** Check if a directory has expandable subdirs */
  const hasSubdirs = useCallback(
    (dir: string): boolean => {
      const subdirs = topLevelSubdirs[dir];
      return subdirs !== undefined && subdirs.length > 0;
    },
    [topLevelSubdirs]
  );

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

      <div className="selection-header">
        <div className="include-root-toggle">
          <label className="vintage-toggle">
            <input
              id="include-root-checkbox"
              type="checkbox"
              checked={selection.includeRoot}
              onChange={handleIncludeRootChange}
            />
            <span className="vintage-toggle-track" />
          </label>
          <label className="toggle-label" htmlFor="include-root-checkbox">
            Include root files
          </label>
        </div>
      </div>

      <div className="dir-selection">
        <div className="dir-selection-header">
          <span>Directories</span>
          <div className="dir-selection-actions">
            <button
              className="dir-action-btn"
              onClick={handleSelectAll}
              disabled={topLevelDirs.length === 0 || allSelected}
            >
              All
            </button>
            <button
              className="dir-action-btn"
              onClick={handleSelectNone}
              disabled={topLevelDirs.length === 0 || noneSelected}
            >
              None
            </button>
          </div>
        </div>
        <div className="dir-checkbox-list">
          {topLevelDirs.map((dir) => {
            const checkboxId = `dir-checkbox-${dir.replace(/[^a-zA-Z0-9]/g, "-")}`;
            const isSelected = selectedDirs.has(dir);
            const isExpanded = expandedDirs.has(dir);
            const canExpand = hasSubdirs(dir);
            const subdirs = topLevelSubdirs[dir] ?? [];
            const hasExclusions = hasExcludedSubdirs(dir);

            return (
              <div key={dir} className="dir-item">
                <div className="dir-row">
                  {canExpand ? (
                    <button
                      className="dir-expand-btn"
                      onClick={() => { handleToggleExpand(dir); }}
                      aria-label={isExpanded ? "Collapse" : "Expand"}
                      aria-expanded={isExpanded}
                    >
                      {isExpanded ? "▼" : "▶"}
                    </button>
                  ) : (
                    <span className="dir-expand-spacer" />
                  )}
                  <IndeterminateCheckbox
                    id={checkboxId}
                    checked={isSelected}
                    indeterminate={isSelected && hasExclusions}
                    onChange={() => { handleDirToggle(dir); }}
                  />
                  <label className="dir-label" htmlFor={checkboxId}>{dir}</label>
                </div>
                {isExpanded && subdirs.length > 0 && (
                  <BundleSubdirTree
                    parentDir={dir}
                    subdirs={subdirs}
                    isParentSelected={isSelected}
                    excludedSubdirs={excludedSubdirs}
                    onSubdirToggle={handleSubdirToggle}
                  />
                )}
              </div>
            );
          })}
          {topLevelDirs.length === 0 && (
            <span className="no-dirs">No directories found</span>
          )}
        </div>
      </div>

      {lastBuildError && (
        <div className="build-error">{lastBuildError}</div>
      )}
    </div>
  );
}
