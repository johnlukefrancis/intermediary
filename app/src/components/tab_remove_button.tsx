// Path: app/src/components/tab_remove_button.tsx
// Description: "x" button for removing repos with confirmation

import type React from "react";
import { useCallback, useState } from "react";
import { ConfirmModal } from "./confirm_modal.js";
import { useConfig } from "../hooks/use_config.js";

interface TabRemoveButtonProps {
  repoId: string;
  label: string;
  onRemoved?: () => void;
  className?: string;
}

export function TabRemoveButton({
  repoId,
  label,
  onRemoved,
  className = "",
}: TabRemoveButtonProps): React.JSX.Element {
  const { removeRepo } = useConfig();
  const [showConfirm, setShowConfirm] = useState(false);

  const handleClick = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    setShowConfirm(true);
  }, []);

  const handleConfirm = useCallback(() => {
    removeRepo(repoId);
    setShowConfirm(false);
    onRemoved?.();
  }, [repoId, removeRepo, onRemoved]);

  const handleCancel = useCallback(() => {
    setShowConfirm(false);
  }, []);

  return (
    <>
      <button
        type="button"
        className={`tab-remove-button ${className}`}
        onClick={handleClick}
        aria-label={`Remove ${label}`}
        title={`Remove ${label}`}
      >
        ×
      </button>
      {showConfirm && (
        <ConfirmModal
          title="Remove Repository"
          message={`Remove "${label}" from Intermediary? This will not delete any files.`}
          confirmLabel="Remove"
          isDestructive
          onConfirm={handleConfirm}
          onCancel={handleCancel}
        />
      )}
    </>
  );
}
