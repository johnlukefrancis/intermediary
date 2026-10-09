// Path: app/src/components/empty_state_panel.tsx
// Description: Shared centered glass empty-state surface with optional details and an action

import type React from "react";
import "../styles/empty_state_panel.css";

interface EmptyStatePanelProps {
  title: string;
  description: string;
  details?: string | null;
  action?: React.ReactNode;
}

export function EmptyStatePanel({ title, description, details, action }: EmptyStatePanelProps): React.JSX.Element {
  return (
    <div className="empty-state-panel">
      <div className="empty-state-panel__surface glass-surface">
        <div className="empty-state-panel__content">
          <p className="empty-state-panel__title">{title}</p>
          <p className="empty-state-panel__description">{description}</p>
          {details && <p className="empty-state-panel__description">{details}</p>}
          {action && <div className="empty-state-panel__action">{action}</div>}
        </div>
      </div>
    </div>
  );
}
