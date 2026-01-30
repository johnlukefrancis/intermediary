// Path: app/src/components/drag_error_notice.tsx
// Description: Small inline error notice for drag failures

import type React from "react";
import "../styles/drag_error_notice.css";

interface DragErrorNoticeProps {
  message: string;
  onDismiss: () => void;
}

export function DragErrorNotice({
  message,
  onDismiss,
}: DragErrorNoticeProps): React.JSX.Element {
  return (
    <div className="drag-error-notice" role="status">
      <span className="drag-error-text">{message}</span>
      <button
        type="button"
        className="drag-error-dismiss"
        onClick={onDismiss}
        aria-label="Dismiss drag error"
      >
        ×
      </button>
    </div>
  );
}
