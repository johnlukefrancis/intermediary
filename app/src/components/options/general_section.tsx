// Path: app/src/components/options/general_section.tsx
// Description: Options panel section for general app settings

import type React from "react";
import type { UiMode } from "../../shared/config.js";
import {
  TriStateRocker,
  type TriStateOption,
} from "./controls/tri_state_rocker.js";
import { OptionsFieldRow } from "./layout/options_field_row.js";

const STANDARD_ICON = (
  <svg
    viewBox="0 0 24 16"
    className="mode-silhouette"
    aria-hidden="true"
    fill="none"
  >
    <rect x="1.5" y="2" width="21" height="12" rx="1.5" stroke="currentColor" />
    <path
      d="M5 6h5M14 6h5M5 10h5M14 10h5"
      stroke="currentColor"
      strokeLinecap="round"
    />
  </svg>
);

const HANDSET_ICON = (
  <svg
    viewBox="0 0 24 16"
    className="mode-silhouette"
    aria-hidden="true"
    fill="none"
  >
    <rect x="8" y="1.5" width="8" height="13" rx="2" stroke="currentColor" />
    <path d="M10 4h4" stroke="currentColor" strokeLinecap="round" />
    <path d="M10 8h4M10 10h4" stroke="currentColor" strokeLinecap="round" />
    <circle cx="12" cy="12.5" r="0.8" fill="currentColor" stroke="none" />
  </svg>
);

const UI_MODES: ReadonlyArray<TriStateOption<UiMode>> = [
  {
    value: "standard",
    label: "STANDARD",
    icon: STANDARD_ICON,
    title: "Standard layout",
  },
  {
    value: "handset",
    label: "HANDSET",
    icon: HANDSET_ICON,
    title: "Handset layout",
  },
];

interface GeneralSectionProps {
  uiMode: UiMode;
  setUiMode: (mode: UiMode) => void;
  windowOpacityPercent: number;
  setWindowOpacityPercent: (value: number) => void;
  textureIntensityPercent: number;
  setTextureIntensityPercent: (value: number) => void;
  autoStageOnChange: boolean;
  setAutoStageOnChange: (value: boolean) => void;
  recentFilesLimit: number;
  setRecentFilesLimit: (value: number) => void;
}

export function GeneralSection({
  uiMode,
  setUiMode,
  windowOpacityPercent,
  setWindowOpacityPercent,
  textureIntensityPercent,
  setTextureIntensityPercent,
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
        title="Preferred layout baseline; runtime mode can auto-switch between standard and handset based on window size"
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
        label="Window opacity"
        title="Global window surface transparency. Lower values increase desktop show-through for a terminal-like overlay effect"
        hint={`${windowOpacityPercent}%`}
        controlAlign="stretch"
        control={(
          <div className="options-range-control">
            <input
              type="range"
              className="options-range-input"
              min={0}
              max={100}
              step={1}
              value={windowOpacityPercent}
              onChange={(event) => {
                const parsed = parseInt(event.target.value, 10);
                if (!Number.isNaN(parsed)) {
                  setWindowOpacityPercent(parsed);
                }
              }}
            />
            <span className="options-range-value" aria-live="polite">
              {windowOpacityPercent}%
            </span>
          </div>
        )}
      />
      <OptionsFieldRow
        label="Texture intensity"
        title="Global substrate texture strength. Independent from window opacity"
        hint={`${textureIntensityPercent}%`}
        controlAlign="stretch"
        control={(
          <div className="options-range-control">
            <input
              type="range"
              className="options-range-input"
              min={0}
              max={100}
              step={1}
              value={textureIntensityPercent}
              onChange={(event) => {
                const parsed = parseInt(event.target.value, 10);
                if (!Number.isNaN(parsed)) {
                  setTextureIntensityPercent(parsed);
                }
              }}
            />
            <span className="options-range-value" aria-live="polite">
              {textureIntensityPercent}%
            </span>
          </div>
        )}
      />
      <OptionsFieldRow
        label="Auto-stage"
        title="When enabled, changed files are instantly copied to the staging folder so they're ready for drag-and-drop"
        controlAlign="start"
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
