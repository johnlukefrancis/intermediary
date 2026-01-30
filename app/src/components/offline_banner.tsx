// Path: app/src/components/offline_banner.tsx
// Description: Connection status banner shown when agent is offline

import type React from "react";
import { useAgent } from "../hooks/use_agent.js";
import "../styles/offline_banner.css";

export function OfflineBanner(): React.JSX.Element | null {
  const { connectionState } = useAgent();

  if (connectionState.status === "connected") {
    return null;
  }

  const isReconnecting = connectionState.status === "reconnecting";
  const message = isReconnecting
    ? `Reconnecting... (attempt ${connectionState.reconnectAttempts})`
    : connectionState.status === "connecting"
      ? "Connecting to agent..."
      : "Agent offline";

  return (
    <div className={`offline-banner ${isReconnecting ? "reconnecting" : ""}`}>
      <span className="offline-icon">{isReconnecting ? "..." : "!"}</span>
      <span className="offline-message">{message}</span>
      {connectionState.lastError && (
        <span className="offline-error">{connectionState.lastError}</span>
      )}
    </div>
  );
}
