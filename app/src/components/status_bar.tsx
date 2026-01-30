// Path: app/src/components/status_bar.tsx
// Description: Status bar with auto-stage toggle

import type React from "react";
import { useAgent } from "../hooks/use_agent.js";
import "../styles/status_bar.css";

export function StatusBar(): React.JSX.Element {
  const { autoStageOnChange, setAutoStageOnChange, connectionState } = useAgent();
  const isConnected = connectionState.status === "connected";

  return (
    <div className="status-bar">
      <div className="status-group">
        <span className="status-label">Auto-stage</span>
        <label className="status-toggle">
          <input
            type="checkbox"
            checked={autoStageOnChange}
            onChange={(event) => {
              setAutoStageOnChange(event.target.checked);
            }}
          />
          <span className="status-toggle-track" aria-hidden="true" />
        </label>
      </div>
      {!isConnected && (
        <span className="status-hint">Applies on reconnect</span>
      )}
    </div>
  );
}
