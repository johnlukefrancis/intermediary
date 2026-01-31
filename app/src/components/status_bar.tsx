// Path: app/src/components/status_bar.tsx
// Description: Status bar with auto-stage toggle, connection status LED, staging path, and error display

import { useCallback, useState } from "react";
import type React from "react";
import { useAgent } from "../hooks/use_agent.js";
import type { ConnectionStatus } from "../lib/agent/connection_state.js";
import "../styles/status_bar.css";

interface ConnectionDisplay {
  cssClass: string;
  label: string;
  showPulse: boolean;
}

function getConnectionDisplay(status: ConnectionStatus, attempts: number): ConnectionDisplay {
  switch (status) {
    case "connected":
      return { cssClass: "connected", label: "Connected", showPulse: false };
    case "connecting":
      return { cssClass: "connecting", label: "Connecting...", showPulse: true };
    case "reconnecting":
      return { cssClass: "reconnecting", label: `Reconnecting (${attempts})`, showPulse: true };
    case "disconnected":
      return { cssClass: "disconnected", label: "Offline", showPulse: false };
  }
}

export function StatusBar(): React.JSX.Element {
  const { autoStageOnChange, setAutoStageOnChange, connectionState, appPaths, helloState } =
    useAgent();
  const [copyFeedback, setCopyFeedback] = useState(false);

  const { status, reconnectAttempts, lastError: connectionError } = connectionState;
  const display = getConnectionDisplay(status, reconnectAttempts);
  const stagingPath = appPaths?.stagingWindowsRoot ?? null;

  // Show connection error when not connected, hello error when connected
  const errorToShow = status === "connected" ? helloState.lastError : connectionError;

  const handleCopyPath = useCallback(() => {
    if (!stagingPath) return;
    void navigator.clipboard.writeText(stagingPath).then(() => {
      setCopyFeedback(true);
      setTimeout(() => {
        setCopyFeedback(false);
      }, 1500);
    });
  }, [stagingPath]);

  return (
    <div className="status-bar">
      <div className="status-left">
        <div className="status-group">
          <span className="status-label">Auto-stage</span>
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
        <span className="chrome-sep" aria-hidden="true">·</span>
        <span className={`status-connection ${display.cssClass}${display.showPulse ? " pulsing" : ""}`}>
          <span className="led-dot" aria-hidden="true" />
          <span className="led-label">Agent: {display.label}</span>
        </span>
        {errorToShow && (
          <>
            <span className="chrome-sep" aria-hidden="true">·</span>
            <span className="status-error" title={errorToShow}>
              {errorToShow.length > 50 ? `${errorToShow.slice(0, 50)}...` : errorToShow}
            </span>
          </>
        )}
      </div>

      <div className="status-right">
        {stagingPath && (
          <button
            type="button"
            className={`path-chip ${copyFeedback ? "copied" : ""}`}
            onClick={handleCopyPath}
            title="Click to copy staging path"
          >
            {copyFeedback ? "Copied!" : stagingPath}
          </button>
        )}
      </div>
    </div>
  );
}
