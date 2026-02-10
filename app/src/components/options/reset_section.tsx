// Path: app/src/components/options/reset_section.tsx
// Description: Options panel reset settings section with confirmation modal

import { useState } from "react";
import type React from "react";
import { ConfirmModal } from "../confirm_modal.js";
import { OptionsFieldRow } from "./layout/options_field_row.js";

interface ResetSectionProps {
  resetConfig: () => void;
}

export function ResetSection({ resetConfig }: ResetSectionProps): React.JSX.Element {
  const [showResetConfirm, setShowResetConfirm] = useState(false);

  return (
    <div className="options-section">
      <div className="options-section-title">Reset</div>
      <OptionsFieldRow
        label="Factory reset"
        controlAlign="stretch"
        control={(
          <div className="options-reset-stack">
            <span className="options-hint">
              Removes repos from Intermediary and clears settings, staged
              bundles, and recent-file caches. It never deletes files inside
              your repositories.
            </span>
            <div className="options-button-row">
              <button
                type="button"
                className="options-button options-button--destructive"
                onClick={() => {
                  setShowResetConfirm(true);
                }}
              >
                Reset all settings
              </button>
            </div>
          </div>
        )}
      />

      {showResetConfirm && (
        <ConfirmModal
          title="Reset All Settings"
          message="Reset all settings? This removes repos from Intermediary and clears bundle selections, staged bundles, and recent-file caches. It never deletes files inside your repositories."
          confirmLabel="Reset"
          isDestructive
          onConfirm={() => {
            resetConfig();
            setShowResetConfirm(false);
          }}
          onCancel={() => {
            setShowResetConfirm(false);
          }}
        />
      )}
    </div>
  );
}
