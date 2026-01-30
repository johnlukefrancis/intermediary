// Path: app/src/components/bundles/preset_selector.tsx
// Description: Preset tabs/buttons for bundle building

import type React from "react";
import type { BundlePresetState } from "../../hooks/use_bundle_state.js";

interface PresetSelectorProps {
  presets: Map<string, BundlePresetState>;
  activePresetId: string;
  onSelect: (presetId: string) => void;
}

export function PresetSelector({
  presets,
  activePresetId,
  onSelect,
}: PresetSelectorProps): React.JSX.Element {
  const presetList = Array.from(presets.values());

  // Single preset: no tabs needed
  if (presetList.length <= 1) {
    return <></>;
  }

  return (
    <div className="preset-selector">
      {presetList.map((preset) => (
        <button
          key={preset.presetId}
          className={`preset-tab ${preset.presetId === activePresetId ? "active" : ""}`}
          onClick={() => { onSelect(preset.presetId); }}
        >
          {preset.presetName}
        </button>
      ))}
    </div>
  );
}
