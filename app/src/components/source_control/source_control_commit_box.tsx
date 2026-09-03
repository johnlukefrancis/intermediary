// Path: app/src/components/source_control/source_control_commit_box.tsx
// Description: Commit message textarea (Ctrl+Enter) and compact COMMIT button for the Source Control column

import type React from "react";
import { useCallback } from "react";

interface SourceControlCommitBoxProps {
  message: string;
  branch: string;
  canCommit: boolean;
  isCommitting: boolean;
  /** Any action pending: the textarea stays editable, the button does not */
  disabled: boolean;
  hint: string | null;
  onMessageChange: (message: string) => void;
  onCommit: () => void;
}

export function SourceControlCommitBox({
  message,
  branch,
  canCommit,
  isCommitting,
  disabled,
  hint,
  onMessageChange,
  onCommit,
}: SourceControlCommitBoxProps): React.JSX.Element {
  const handleKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (event.key !== "Enter" || !(event.ctrlKey || event.metaKey)) return;
      event.preventDefault();
      if (canCommit) onCommit();
    },
    [canCommit, onCommit]
  );

  const buttonTitle = isCommitting
    ? "Commit in progress"
    : hint ?? (message.trim().length === 0 ? "Enter a commit message" : `Commit to ${branch}`);

  return (
    <div className="source-control-commit">
      <textarea
        className="source-control-commit__message"
        value={message}
        rows={3}
        placeholder={`Message (Ctrl+Enter to commit on ${branch})`}
        aria-label="Commit message"
        spellCheck={false}
        disabled={isCommitting}
        onChange={(event) => { onMessageChange(event.target.value); }}
        onKeyDown={handleKeyDown}
      />
      <button
        type="button"
        className={`build-button source-control-commit__button${isCommitting ? " building" : ""}`}
        disabled={disabled || !canCommit}
        aria-busy={isCommitting}
        title={buttonTitle}
        onClick={onCommit}
      >
        {isCommitting ? "COMMITTING…" : "COMMIT"}
        {isCommitting && (
          <span className="build-button-progress indeterminate" aria-hidden="true" />
        )}
      </button>
      {hint !== null && !isCommitting && (
        <span className="source-control-commit__hint">{hint}</span>
      )}
    </div>
  );
}
