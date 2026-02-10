// Path: app/src/components/options/general_section.tsx
// Description: Options panel section for general app settings

import type React from "react";
import type { UiMode } from "../../shared/config.js";

const UI_MODES: ReadonlyArray<{ value: UiMode; label: string }> = [
  { value: "standard", label: "STANDARD" },
  { value: "compact", label: "COMPACT" },
  { value: "handset", label: "HANDSET" },
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
      <div className="options-row" title="Layout density for the UI">
        <span className="options-row-label">Mode</span>
        <div role="radiogroup" className="segmented-switcher" aria-label="UI mode">
          {UI_MODES.map(({ value, label }, index) => (
            <button
              key={value}
              type="button"
              role="radio"
              aria-checked={uiMode === value}
              tabIndex={uiMode === value ? 0 : -1}
              data-ui-mode-option={value}
              className={`segmented-option${uiMode === value ? " active" : ""}`}
              onClick={() => {
                setUiMode(value);
              }}
              onKeyDown={(event) => {
                let nextMode: UiMode | null = null;
                if (event.key === "ArrowRight" || event.key === "ArrowDown") {
                  const next = UI_MODES[(index + 1) % UI_MODES.length];
                  nextMode = next?.value ?? null;
                } else if (event.key === "ArrowLeft" || event.key === "ArrowUp") {
                  const prev = UI_MODES[(index - 1 + UI_MODES.length) % UI_MODES.length];
                  nextMode = prev?.value ?? null;
                } else if (event.key === "Home") {
                  nextMode = UI_MODES[0]?.value ?? null;
                } else if (event.key === "End") {
                  nextMode = UI_MODES[UI_MODES.length - 1]?.value ?? null;
                }

                if (!nextMode) {
                  return;
                }

                event.preventDefault();
                setUiMode(nextMode);
                const container = event.currentTarget.parentElement;
                const nextButton = container?.querySelector<HTMLButtonElement>(
                  `[data-ui-mode-option="${nextMode}"]`
                );
                nextButton?.focus();
              }}
            >
              {label}
            </button>
          ))}
        </div>
      </div>
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
