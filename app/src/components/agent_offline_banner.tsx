// Path: app/src/components/agent_offline_banner.tsx
// Description: Banner with diagnostics when the WSL agent is offline

import type React from "react";
import { useAgent } from "../hooks/use_agent.js";
import "../styles/agent_offline_banner.css";

export function AgentOfflineBanner(): React.JSX.Element | null {
  const { agentDiagnostics } = useAgent();

  if (!agentDiagnostics) {
    return null;
  }

  const {
    expectedUrl,
    probeListening,
    probeError,
    lastError,
    configuredHost,
    port,
  } = agentDiagnostics;

  const listeningLabel =
    probeListening === null ? "unknown" : probeListening ? "yes" : "no";
  const listeningClass =
    probeListening === null
      ? "agent-offline-banner__value--unknown"
      : probeListening
        ? "agent-offline-banner__value--ok"
        : "agent-offline-banner__value--bad";

  const isNonLoopbackHost =
    configuredHost !== "127.0.0.1" && configuredHost !== "localhost";

  return (
    <div className="agent-offline-banner glass-surface" role="status" aria-live="polite">
      <div className="agent-offline-banner__title">WSL agent offline</div>
      <div className="agent-offline-banner__details">
        <span className="agent-offline-banner__detail">
          URL:{" "}
          <span className="agent-offline-banner__mono">{expectedUrl}</span>
        </span>
        <span className={`agent-offline-banner__detail ${listeningClass}`}>
          Port listening: {listeningLabel}
        </span>
        {probeError && (
          <span className="agent-offline-banner__detail">
            Probe error: {probeError}
          </span>
        )}
        {lastError && (
          <span className="agent-offline-banner__detail">
            Last error: {lastError}
          </span>
        )}
        {isNonLoopbackHost && (
          <span className="agent-offline-banner__detail">
            Configured host: {configuredHost} (only ws://127.0.0.1:{port} is
            allowed)
          </span>
        )}
      </div>
    </div>
  );
}
