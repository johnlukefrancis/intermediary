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
  topLevelFiles: string[];
  onDragStart: (hostPath: string) => Promise<void>;
  onOpenFile: (path: string) => void;
  emptyMessage?: string;
}

export function BundleColumn({
  repoId,
  bundleState,
  topLevelFiles,
  onDragStart,
  onOpenFile,
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
        repoId={repoId}
        selection={activePreset.selection}
        topLevelDirs={bundleState.topLevelDirs}
        topLevelFiles={topLevelFiles}
        isBuilding={activePreset.isBuilding}
        isCancelling={activePreset.isCancelling}
        buildProgress={activePreset.buildProgress}
        lastBuildError={activePreset.lastBuildError}
        onSelectionChange={(sel) => { bundleState.setSelection(activePreset.presetId, sel); }}
        onBuild={() => { void bundleState.buildBundle(activePreset.presetId); }}
        onCancelBuild={() => { void bundleState.cancelBundleBuild(activePreset.presetId); }}
        onOpenFile={onOpenFile}
      />

      <BundleList
        bundles={activePreset.bundles}
        onDragStart={onDragStart}
        emptyMessage={emptyMessage}
        freshlyBuiltAt={activePreset.freshlyBuiltAt}
      />
    </div>
  );
}
