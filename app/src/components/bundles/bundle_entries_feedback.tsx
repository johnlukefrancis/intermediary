// Path: app/src/components/bundles/bundle_entries_feedback.tsx
// Description: Replace-conflict confirmation and inline error notice, shared by import and entry-action transactions

import type React from "react";
import { ConfirmModal } from "../confirm_modal.js";

const MAX_LISTED_CONFLICTS = 8;

export interface EntriesFeedbackReplace {
  conflicts: string[];
}

interface BundleEntriesFeedbackProps {
  /** Distinguishes the two call sites in the alert heading and dismiss affordance. */
  label: string;
  pendingReplace: EntriesFeedbackReplace | null;
  error: string | null;
  onConfirmReplace: () => void;
  onCancelReplace: () => void;
  onDismissError: () => void;
}

function replaceTitle(count: number): string {
  return `Replace ${count} existing file${count === 1 ? "" : "s"}?`;
}

function replaceMessage(conflicts: string[]): string {
  const shown = conflicts.slice(0, MAX_LISTED_CONFLICTS);
  const remaining = conflicts.length - shown.length;
  const list = remaining > 0 ? `${shown.join("\n")}\n+${remaining} more` : shown.join("\n");
  return `These files will be overwritten:\n${list}`;
}

export function BundleEntriesFeedback({
  label,
  pendingReplace,
  error,
  onConfirmReplace,
  onCancelReplace,
  onDismissError,
}: BundleEntriesFeedbackProps): React.JSX.Element {
  return (
    <>
      {pendingReplace && (
        <ConfirmModal
          title={replaceTitle(pendingReplace.conflicts.length)}
          message={replaceMessage(pendingReplace.conflicts)}
          confirmLabel="Replace"
          isDestructive
          onConfirm={onConfirmReplace}
          onCancel={onCancelReplace}
        />
      )}
      {error && (
        <div className="build-error source-control-notice" role="alert">
          <span className="source-control-notice__heading">&gt; {label} FAILED</span>
          <span className="source-control-notice__message">{error}</span>
          <button type="button" className="dir-action-btn" onClick={onDismissError}>
            Dismiss
          </button>
        </div>
      )}
    </>
  );
}
