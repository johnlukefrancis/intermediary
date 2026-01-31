// Path: app/src/components/options_overlay.tsx
// Description: Full-screen transparent overlay with options panel for app settings

import type React from "react";
import { useCallback, useState } from "react";
import { createPortal } from "react-dom";
import "../styles/options_overlay.css";

interface OptionsOverlayProps {
  autoStageOnChange: boolean;
  setAutoStageOnChange: (value: boolean) => void;
  stagingPath: string | null;
  onClose: () => void;
}

export function OptionsOverlay({
  autoStageOnChange,
  setAutoStageOnChange,
  stagingPath,
  onClose,
}: OptionsOverlayProps): React.JSX.Element {
  const [copyFeedback, setCopyFeedback] = useState(false);

  const handleBackdropClick = useCallback(
    (event: React.MouseEvent<HTMLDivElement>) => {
      if (event.target === event.currentTarget) {
        onClose();
      }
    },
    [onClose]
  );

  const handleCopyPath = useCallback(() => {
    if (!stagingPath) return;
    void navigator.clipboard.writeText(stagingPath).then(() => {
      setCopyFeedback(true);
      setTimeout(() => {
        setCopyFeedback(false);
      }, 1500);
    });
  }, [stagingPath]);

  return createPortal(
    <div className="options-overlay" onClick={handleBackdropClick}>
      <div className="options-panel">
        <button
          type="button"
          className="options-close-button"
          onClick={onClose}
          aria-label="Close options"
        >
          ×
        </button>

        <div className="options-row">
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

        {stagingPath && (
          <div className="options-row stacked">
            <span className="options-row-label">Staging Path</span>
            <button
              type="button"
              className={`path-chip ${copyFeedback ? "copied" : ""}`}
              onClick={handleCopyPath}
              title="Click to copy staging path"
            >
              {copyFeedback ? "Copied!" : stagingPath}
            </button>
          </div>
        )}
      </div>
    </div>,
    document.body
  );
}
