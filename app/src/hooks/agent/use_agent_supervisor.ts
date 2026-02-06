// Path: app/src/hooks/agent/use_agent_supervisor.ts
// Description: Manage auto-start and restart of host + optional WSL agents

import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ConnectionState } from "../../lib/agent/connection_state.js";
import type { AgentSupervisorResult } from "../../types/agent_supervisor.js";

const ENSURE_THROTTLE_MS = 1500;
const WSL_HEALTHCHECK_MS = 10000;

interface UseAgentSupervisorOptions {
  configIsLoaded: boolean;
  agentHost: string;
  agentPort: number;
  requiresWsl: boolean;
  autoStartEnabled: boolean;
  distroOverride: string | null;
  connectionState: ConnectionState;
}

interface UseAgentSupervisorResult {
  supervisorResult: AgentSupervisorResult | null;
  supervisorError: string | null;
  restartAgent: () => void;
}

function toSupervisorErrorMessage(err: unknown, fallback: string): string {
  if (typeof err === "string" && err.trim().length > 0) {
    return err;
  }
  if (err instanceof Error && err.message.trim().length > 0) {
    return err.message;
  }
  if (
    typeof err === "object" &&
    err !== null &&
    "message" in err
  ) {
    const maybeMessage = (err as { message?: unknown }).message;
    if (typeof maybeMessage === "string" && maybeMessage.trim().length > 0) {
      return maybeMessage;
    }
  }
  return fallback;
}

interface SupervisorInputsSnapshot {
  configIsLoaded: boolean;
  agentHost: string;
  agentPort: number;
  requiresWsl: boolean;
  autoStartEnabled: boolean;
  distroOverride: string | null;
}

export function useAgentSupervisor({
  configIsLoaded,
  agentHost,
  agentPort,
  requiresWsl,
  autoStartEnabled,
  distroOverride,
  connectionState,
}: UseAgentSupervisorOptions): UseAgentSupervisorResult {
  const [supervisorResult, setSupervisorResult] = useState<AgentSupervisorResult | null>(
    null
  );
  const [supervisorError, setSupervisorError] = useState<string | null>(null);
  const lastEnsureRef = useRef<number>(0);
  const lastRequiresWslRef = useRef<boolean | null>(null);
  const ensureInFlightRef = useRef(false);
  const ensureQueuedRef = useRef(false);
  const latestInputsRef = useRef<SupervisorInputsSnapshot>({
    configIsLoaded,
    agentHost,
    agentPort,
    requiresWsl,
    autoStartEnabled,
    distroOverride,
  });

  useEffect(() => {
    latestInputsRef.current = {
      configIsLoaded,
      agentHost,
      agentPort,
      requiresWsl,
      autoStartEnabled,
      distroOverride,
    };
  }, [
    configIsLoaded,
    agentHost,
    agentPort,
    requiresWsl,
    autoStartEnabled,
    distroOverride,
  ]);

  const ensureAgentRunning = useCallback(
    (ignoreThrottle = false) => {
      const inputs = latestInputsRef.current;
      if (!inputs.configIsLoaded || !inputs.autoStartEnabled) return;

      const isLoopback =
        inputs.agentHost === "127.0.0.1" || inputs.agentHost === "localhost";
      if (!isLoopback) {
        setSupervisorError("Auto-start requires agentHost 127.0.0.1");
        return;
      }

      const now = Date.now();
      if (!ignoreThrottle && now - lastEnsureRef.current < ENSURE_THROTTLE_MS) {
        return;
      }
      lastEnsureRef.current = now;
      if (ensureInFlightRef.current) {
        ensureQueuedRef.current = true;
        return;
      }
      ensureInFlightRef.current = true;
      const requestConfig = {
        port: inputs.agentPort,
        autoStart: inputs.autoStartEnabled,
        distro: inputs.distroOverride,
        requiresWsl: inputs.requiresWsl,
      };

      invoke<AgentSupervisorResult>("ensure_agent_running", {
        config: requestConfig,
      })
        .then((result) => {
          lastRequiresWslRef.current = requestConfig.requiresWsl;
          setSupervisorResult(result);
          setSupervisorError(
            result.status === "backoff" ? result.message ?? null : null
          );
        })
        .catch((err: unknown) => {
          const message = toSupervisorErrorMessage(err, "Agent start failed");
          setSupervisorError(message);
        })
        .finally(() => {
          ensureInFlightRef.current = false;
          if (!ensureQueuedRef.current) {
            return;
          }
          ensureQueuedRef.current = false;
          setTimeout(() => {
            ensureAgentRunning(true);
          }, 0);
        });
    },
    []
  );

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

    const requiresWslChanged = lastRequiresWslRef.current !== requiresWsl;

    if (connectionState.status === "connected" && !requiresWslChanged) {
      return;
    }

    ensureAgentRunning(requiresWslChanged);
  }, [
    configIsLoaded,
    autoStartEnabled,
    agentHost,
    agentPort,
    requiresWsl,
    distroOverride,
    connectionState.status,
    connectionState.reconnectAttempts,
    ensureAgentRunning,
  ]);

  useEffect(() => {
    if (!configIsLoaded || !autoStartEnabled) return;
    if (!requiresWsl) return;
    if (connectionState.status !== "connected") return;

    const interval = setInterval(() => {
      ensureAgentRunning();
    }, WSL_HEALTHCHECK_MS);

    return () => {
      clearInterval(interval);
    };
  }, [
    configIsLoaded,
    autoStartEnabled,
    requiresWsl,
    connectionState.status,
    ensureAgentRunning,
  ]);

  const restartAgent = useCallback(() => {
    if (!configIsLoaded) return;
    invoke<AgentSupervisorResult>("restart_agent", {
      config: {
        port: agentPort,
        autoStart: true,
        distro: distroOverride,
        requiresWsl,
      },
    })
      .then((result) => {
        setSupervisorResult(result);
        setSupervisorError(
          result.status === "backoff" ? result.message ?? null : null
        );
      })
      .catch((err: unknown) => {
        const message = toSupervisorErrorMessage(err, "Agent restart failed");
        setSupervisorError(message);
      });
  }, [agentPort, configIsLoaded, distroOverride, requiresWsl]);

  return { supervisorResult, supervisorError, restartAgent };
}
