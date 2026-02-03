// Path: app/src/hooks/agent/use_agent_supervisor.ts
// Description: Manage auto-start and restart of the WSL agent

import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ConnectionState } from "../../lib/agent/connection_state.js";
import type { AgentSupervisorResult } from "../../types/agent_supervisor.js";

interface UseAgentSupervisorOptions {
  configIsLoaded: boolean;
  agentHost: string;
  agentPort: number;
  autoStartEnabled: boolean;
  distroOverride: string | null;
  connectionState: ConnectionState;
}

interface UseAgentSupervisorResult {
  supervisorResult: AgentSupervisorResult | null;
  supervisorError: string | null;
  restartAgent: () => void;
}

export function useAgentSupervisor({
  configIsLoaded,
  agentHost,
  agentPort,
  autoStartEnabled,
  distroOverride,
  connectionState,
}: UseAgentSupervisorOptions): UseAgentSupervisorResult {
  const [supervisorResult, setSupervisorResult] = useState<AgentSupervisorResult | null>(
    null
  );
  const [supervisorError, setSupervisorError] = useState<string | null>(null);
  const lastEnsureRef = useRef<number>(0);

  useEffect(() => {
    if (!configIsLoaded) return;

    if (!autoStartEnabled) {
      setSupervisorResult(null);
      setSupervisorError(null);
      return;
    }

    const isLoopback = agentHost === "127.0.0.1" || agentHost === "localhost";
    if (!isLoopback) {
      setSupervisorError("Auto-start requires agentHost 127.0.0.1");
      return;
    }

    if (connectionState.status === "connected") {
      return;
    }

    const now = Date.now();
    if (now - lastEnsureRef.current < 1500) {
      return;
    }
    lastEnsureRef.current = now;

    invoke<AgentSupervisorResult>("ensure_agent_running", {
      config: {
        port: agentPort,
        autoStart: autoStartEnabled,
        distro: distroOverride,
      },
    })
      .then((result) => {
        setSupervisorResult(result);
        setSupervisorError(
          result.status === "backoff" ? result.message ?? null : null
        );
      })
      .catch((err: unknown) => {
        const message = err instanceof Error ? err.message : "Agent start failed";
        setSupervisorError(message);
      });
  }, [
    configIsLoaded,
    autoStartEnabled,
    agentHost,
    agentPort,
    distroOverride,
    connectionState.status,
    connectionState.reconnectAttempts,
  ]);

  const restartAgent = useCallback(() => {
    if (!configIsLoaded) return;
    invoke<AgentSupervisorResult>("restart_agent", {
      config: {
        port: agentPort,
        autoStart: true,
        distro: distroOverride,
      },
    })
      .then((result) => {
        setSupervisorResult(result);
        setSupervisorError(
          result.status === "backoff" ? result.message ?? null : null
        );
      })
      .catch((err: unknown) => {
        const message = err instanceof Error ? err.message : "Agent restart failed";
        setSupervisorError(message);
      });
  }, [agentPort, configIsLoaded, distroOverride]);

  return { supervisorResult, supervisorError, restartAgent };
}
