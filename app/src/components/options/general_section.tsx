// Path: app/src/components/options/general_section.tsx
// Description: Options panel section for general app settings

import type React from "react";
import type { UiMode } from "../../shared/config.js";
import {
  TriStateRocker,
  type TriStateOption,
} from "./controls/tri_state_rocker.js";
import { OptionsFieldRow } from "./layout/options_field_row.js";

const UI_MODES: ReadonlyArray<TriStateOption<UiMode>> = [
  {
    value: "standard",
    label: "STD",
    icon: "[|||]",
    title: "Standard layout",
  },
  {
    value: "compact",
    label: "CMP",
    icon: "[|| ]",
    title: "Compact layout",
  },
  {
    value: "handset",
    label: "HND",
    icon: "[|  ]",
    title: "Handset layout",
  },
];

interface GeneralSectionProps {
  uiMode: UiMode;
  setUiMode: (mode: UiMode) => void;
  autoStageOnChange: boolean;
  setAutoStageOnChange: (value: boolean) => void;
  recentFilesLimit: number;
  setRecentFilesLimit: (value: number) => void;
}

export function GeneralSection({
  uiMode,
  setUiMode,
  autoStageOnChange,
  setAutoStageOnChange,
  recentFilesLimit,
  setRecentFilesLimit,
}: GeneralSectionProps): React.JSX.Element {
  return (
    <div className="options-section">
      <div className="options-section-title">General</div>
      <OptionsFieldRow
        label="Mode"
        title="Layout density for the UI"
        controlAlign="stretch"
        control={(
          <TriStateRocker
            value={uiMode}
            options={UI_MODES}
            onChange={setUiMode}
            ariaLabel="UI mode"
            className="tri-state-rocker--mode"
          />
        )}
      />
      <OptionsFieldRow
        label="Auto-stage"
        title="When enabled, changed files are instantly copied to the staging folder so they're ready for drag-and-drop"
        control={(
          <label className="vintage-toggle">
            <input
              type="checkbox"
              checked={autoStageOnChange}
              onChange={(event) => {
                setAutoStageOnChange(event.target.checked);
              }}
            />
            <span className="vintage-toggle-track" aria-hidden="true" />
          </label>
        )}
      />
      <OptionsFieldRow
        label="Recent files limit"
        title="Higher values may impact UI performance"
        controlAlign="stretch"
        control={(
          <input
            type="number"
            className="options-number-input"
            value={recentFilesLimit}
            min={25}
            max={2000}
            onChange={(event) => {
              const parsed = parseInt(event.target.value, 10);
              if (!Number.isNaN(parsed)) {
                setRecentFilesLimit(parsed);
              }
            }}
          />
        )}
      />
    </div>
  );
}
