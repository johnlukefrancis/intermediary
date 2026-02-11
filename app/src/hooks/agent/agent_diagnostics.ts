// Path: app/src/hooks/agent/agent_diagnostics.ts
// Description: Agent diagnostics model and helpers for connection-state-driven status bar details

import type { ConnectionState } from "../../lib/agent/connection_state.js";
import type { AgentSupervisorResult } from "../../types/agent_supervisor.js";

export interface AgentDiagnostics {
  expectedUrl: string;
  probeListening: boolean | null;
  probeError: string | null;
  lastError: string | null;
  configuredHost: string;
  port: number;
  autoStartEnabled: boolean;
  supervisorStatus: AgentSupervisorResult["status"] | null;
  supervisorError: string | null;
}

interface BuildAgentDiagnosticsOptions {
  connectionState: ConnectionState;
  expectedUrl: string;
  probeListening: boolean | null;
  probeError: string | null;
  configuredHost: string;
  port: number;
  autoStartEnabled: boolean;
  supervisorStatus: AgentSupervisorResult["status"] | null;
  supervisorError: string | null;
  startupGateRequired: boolean;
  startupEnsureState: "idle" | "ensuring" | "ready" | "failed";
  startupEnsureError: string | null;
}

export function buildAgentDiagnostics(
  options: BuildAgentDiagnosticsOptions
): AgentDiagnostics | null {
  const {
    connectionState,
    expectedUrl,
    probeListening,
    probeError,
    configuredHost,
    port,
    autoStartEnabled,
    supervisorStatus,
    supervisorError,
    startupGateRequired,
    startupEnsureState,
    startupEnsureError,
  } = options;

  const shouldShowDiagnostics =
    connectionState.status !== "connected" &&
    (connectionState.reconnectAttempts > 0 ||
      connectionState.lastError !== null ||
      supervisorError !== null);

  if (!shouldShowDiagnostics) {
    return null;
  }

  return {
    expectedUrl,
    probeListening,
    probeError,
    lastError: connectionState.lastError,
    configuredHost,
    port,
    autoStartEnabled,
    supervisorStatus,
    supervisorError:
      supervisorError ??
      (startupGateRequired && startupEnsureState === "failed"
        ? startupEnsureError
        : null),
  };
}

export function hostSupportsWsl(): boolean {
  if (typeof navigator === "undefined") {
    return false;
  }
  return /Windows/i.test(navigator.userAgent);
}
