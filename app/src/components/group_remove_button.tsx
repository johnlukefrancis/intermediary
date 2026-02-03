// Path: app/src/components/group_remove_button.tsx
// Description: Remove button for grouped repos with confirmation

import type React from "react";
import { useCallback, useState } from "react";
import { ConfirmModal } from "./confirm_modal.js";
import { useConfig } from "../hooks/use_config.js";

interface GroupRemoveButtonProps {
  groupId: string;
  groupLabel: string;
  repoCount: number;
  variant: "icon" | "dropdown";
  className?: string;
  onRemoved?: () => void;
}

export function GroupRemoveButton({
  groupId,
  groupLabel,
  repoCount,
  variant,
  className = "",
  onRemoved,
}: GroupRemoveButtonProps): React.JSX.Element {
  const { removeGroup } = useConfig();
  const [showConfirm, setShowConfirm] = useState(false);

  const handleClick = useCallback((event: React.MouseEvent) => {
    event.stopPropagation();
    setShowConfirm(true);
  }, []);

  const handleConfirm = useCallback(() => {
    removeGroup(groupId);
    setShowConfirm(false);
    onRemoved?.();
  }, [groupId, removeGroup, onRemoved]);

  const handleCancel = useCallback(() => {
    setShowConfirm(false);
  }, []);

  const repoLabel = repoCount === 1 ? "repo" : "repos";
  const message = `Remove ${repoCount} ${repoLabel} in group "${groupLabel}" from Intermediary? This will not delete any files.`;

  return (
    <>
      {variant === "icon" ? (
        <button
          type="button"
          className={`tab-remove-button ${className}`}
          onClick={handleClick}
          aria-label={`Remove ${groupLabel} group`}
          title={`Remove ${groupLabel} group`}
        >
          ×
        </button>
      ) : (
        <button
          type="button"
          className={`group-dropdown-remove-group ${className}`}
          onClick={handleClick}
        >
          Remove group
        </button>
      )}
      {showConfirm && (
        <ConfirmModal
          title="Remove Group"
          message={message}
          confirmLabel="Remove"
          isDestructive
          onConfirm={handleConfirm}
          onCancel={handleCancel}
        />
      )}
    </>
  );
}
