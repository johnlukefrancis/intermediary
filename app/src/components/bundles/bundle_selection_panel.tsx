// Path: app/src/components/bundles/bundle_selection_panel.tsx
// Description: Selection UI for bundle building (root toggle, dir checkboxes)

import type React from "react";
import { useCallback } from "react";
import type { BundleSelection } from "../../shared/protocol.js";

interface BundleSelectionPanelProps {
  selection: BundleSelection;
  topLevelDirs: string[];
  isBuilding: boolean;
  lastBuildError: string | null;
  onSelectionChange: (selection: BundleSelection) => void;
  onBuild: () => void;
}

export function BundleSelectionPanel({
  selection,
  topLevelDirs,
  isBuilding,
  lastBuildError,
  onSelectionChange,
  onBuild,
}: BundleSelectionPanelProps): React.JSX.Element {
  const selectedDirs = new Set(selection.topLevelDirs);
  const allSelected =
    topLevelDirs.length > 0 && selection.topLevelDirs.length === topLevelDirs.length;
  const noneSelected = selection.topLevelDirs.length === 0;

  const handleIncludeRootChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      onSelectionChange({ ...selection, includeRoot: e.target.checked });
    },
    [selection, onSelectionChange]
  );

  const handleDirToggle = useCallback(
    (dir: string) => {
      const currentSet = new Set(selection.topLevelDirs);

      if (currentSet.has(dir)) {
        currentSet.delete(dir);
      } else {
        currentSet.add(dir);
      }

      onSelectionChange({
        ...selection,
        topLevelDirs: Array.from(currentSet).sort(),
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

  return (
    <div className="bundle-selection-panel">
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
            return (
              <div key={dir} className="dir-checkbox">
                <label className="vintage-toggle">
                  <input
                    id={checkboxId}
                    type="checkbox"
                    checked={selectedDirs.has(dir)}
                    onChange={() => { handleDirToggle(dir); }}
                  />
                  <span className="vintage-toggle-track" />
                </label>
                <label className="dir-label" htmlFor={checkboxId}>{dir}</label>
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

      <button
        className="build-button"
        onClick={onBuild}
        disabled={isBuilding || (!selection.includeRoot && selection.topLevelDirs.length === 0)}
      >
        {isBuilding ? "Building..." : "Build Bundle"}
      </button>
    </div>
  );
}
