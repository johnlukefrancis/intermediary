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
  // Empty topLevelDirs in selection means ALL
  const selectedDirs = selection.topLevelDirs.length > 0
    ? new Set(selection.topLevelDirs)
    : new Set(topLevelDirs);

  const allSelected = topLevelDirs.every((d) => selectedDirs.has(d));
  const noneSelected = topLevelDirs.every((d) => !selectedDirs.has(d));

  const handleIncludeRootChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      onSelectionChange({ ...selection, includeRoot: e.target.checked });
    },
    [selection, onSelectionChange]
  );

  const handleDirToggle = useCallback(
    (dir: string) => {
      const currentSet = new Set(selection.topLevelDirs.length > 0
        ? selection.topLevelDirs
        : topLevelDirs);

      if (currentSet.has(dir)) {
        currentSet.delete(dir);
      } else {
        currentSet.add(dir);
      }

      // If all selected, use empty array (means ALL)
      const newDirs = currentSet.size === topLevelDirs.length
        ? []
        : Array.from(currentSet).sort();

      onSelectionChange({ ...selection, topLevelDirs: newDirs });
    },
    [selection, topLevelDirs, onSelectionChange]
  );

  const handleSelectAll = useCallback(() => {
    onSelectionChange({ ...selection, topLevelDirs: [] });
  }, [selection, onSelectionChange]);

  const handleSelectNone = useCallback(() => {
    onSelectionChange({ ...selection, topLevelDirs: ["__none__"] });
  }, [selection, onSelectionChange]);

  return (
    <div className="bundle-selection-panel">
      <div className="selection-header">
        <label className="include-root-toggle">
          <input
            type="checkbox"
            checked={selection.includeRoot}
            onChange={handleIncludeRootChange}
          />
          Include root files
        </label>
      </div>

      <div className="dir-selection">
        <div className="dir-selection-header">
          <span>Directories</span>
          <div className="dir-selection-actions">
            <button
              className="dir-action-btn"
              onClick={handleSelectAll}
              disabled={allSelected}
            >
              All
            </button>
            <button
              className="dir-action-btn"
              onClick={handleSelectNone}
              disabled={noneSelected}
            >
              None
            </button>
          </div>
        </div>
        <div className="dir-checkbox-list">
          {topLevelDirs.map((dir) => (
            <label key={dir} className="dir-checkbox">
              <input
                type="checkbox"
                checked={selectedDirs.has(dir)}
                onChange={() => { handleDirToggle(dir); }}
              />
              {dir}
            </label>
          ))}
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
        disabled={isBuilding || (noneSelected && !selection.includeRoot)}
      >
        {isBuilding ? "Building..." : "Build Bundle"}
      </button>
    </div>
  );
}
