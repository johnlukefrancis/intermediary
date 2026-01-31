// Path: app/src/components/status_bar.tsx
// Description: Status bar with auto-stage toggle, staging path, and error display

import { useCallback, useState } from "react";
import type React from "react";
import { useAgent } from "../hooks/use_agent.js";
import "../styles/status_bar.css";

export function StatusBar(): React.JSX.Element {
  const { autoStageOnChange, setAutoStageOnChange, connectionState, appPaths, helloState } =
    useAgent();
  const isConnected = connectionState.status === "connected";
  const [copyFeedback, setCopyFeedback] = useState(false);

  const stagingPath = appPaths?.stagingWindowsRoot ?? null;
  const lastError = helloState.lastError;
  const connectionLabel = isConnected ? "Connected" : "Disconnected";

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
        <span className={`status-connection ${isConnected ? "connected" : "disconnected"}`}>
          <span className="led-dot" aria-hidden="true" />
          <span className="led-label">Agent: {connectionLabel}</span>
        </span>
        {!isConnected && (
          <>
            <span className="chrome-sep" aria-hidden="true">·</span>
            <span className="status-hint">Applies on reconnect</span>
          </>
        )}
      </div>

      <div className="status-right">
        {lastError && (
          <>
            <span className="status-error" title={lastError}>
              Error: {lastError.length > 40 ? `${lastError.slice(0, 40)}...` : lastError}
            </span>
            <span className="chrome-sep" aria-hidden="true">·</span>
          </>
        )}
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
