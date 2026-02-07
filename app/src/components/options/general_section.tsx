// Path: app/src/components/options/general_section.tsx
// Description: Options panel section for general app settings

import type React from "react";

interface GeneralSectionProps {
  autoStageOnChange: boolean;
  setAutoStageOnChange: (value: boolean) => void;
  recentFilesLimit: number;
  setRecentFilesLimit: (value: number) => void;
}

export function GeneralSection({
  autoStageOnChange,
  setAutoStageOnChange,
  recentFilesLimit,
  setRecentFilesLimit,
}: GeneralSectionProps): React.JSX.Element {
  return (
    <div className="options-section">
      <div className="options-section-title">General</div>
      <div
        className="options-row"
        title="When enabled, changed files are instantly copied to the staging folder so they're ready for drag-and-drop"
      >
        <span className="options-row-label">Auto-stage</span>
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
      </div>
      <div
        className="options-row"
        title="Higher values may impact UI performance"
      >
        <span className="options-row-label">Recent files limit</span>
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
      </div>
    </div>
  );
}
