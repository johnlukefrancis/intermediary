// Path: app/src/components/bundles/bundle_column.tsx
// Description: Main bundles column component

import type React from "react";
import { PresetSelector } from "./preset_selector.js";
import { BundleSelectionPanel } from "./bundle_selection_panel.js";
import { BundleList } from "./bundle_list.js";
import type { BundleState } from "../../hooks/use_bundle_state.js";

interface BundleColumnProps {
  repoId: string;
  bundleState: BundleState;
  onDragStart: (windowsPath: string) => Promise<void>;
  emptyMessage?: string;
}

export function BundleColumn({
  repoId: _repoId,
  bundleState,
  onDragStart,
  emptyMessage = "No bundles yet",
}: BundleColumnProps): React.JSX.Element {
  const activePreset = bundleState.presets.get(bundleState.activePresetId);

  if (!activePreset) {
    return (
      <div className="bundle-column">
        <p className="bundle-column-empty">No preset configured</p>
      </div>
    );
  }

  return (
    <div className="bundle-column">
      <PresetSelector
        presets={bundleState.presets}
        activePresetId={bundleState.activePresetId}
        onSelect={bundleState.setActivePreset}
      />

      <BundleSelectionPanel
        selection={activePreset.selection}
        topLevelDirs={bundleState.topLevelDirs}
        isBuilding={activePreset.isBuilding}
        lastBuildError={activePreset.lastBuildError}
        onSelectionChange={(sel) => { bundleState.setSelection(activePreset.presetId, sel); }}
        onBuild={() => { void bundleState.buildBundle(activePreset.presetId); }}
      />

      <BundleList
        bundles={activePreset.bundles}
        onDragStart={onDragStart}
        emptyMessage={emptyMessage}
      />
    </div>
  );
}
