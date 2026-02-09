// Path: app/src/hooks/agent/use_agent_supervisor.ts
// Description: Manage auto-start and restart of host-agent supervision with optional Windows WSL backend
import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ConnectionState } from "../../lib/agent/connection_state.js";
import type { AgentSupervisorResult } from "../../types/agent_supervisor.js";
import {
  buildRequestConfig,
  isLoopbackHost,
  requestRequiresWsl,
  toSupervisorErrorMessage,
  type AgentSupervisorConfigPayload,
  type SupervisorInputsSnapshot,
} from "./use_agent_supervisor_helpers.js";
const ENSURE_THROTTLE_MS = 1500;
const WSL_HEALTHCHECK_MS = 10000;
type StartupEnsureState = "idle" | "ensuring" | "ready" | "failed";
interface UseAgentSupervisorOptions {
  configIsLoaded: boolean;
  agentHost: string;
  agentPort: number;
  platformSupportsWsl: boolean;
  requiresWsl: boolean;
  autoStartEnabled: boolean;
  distroOverride: string | null;
  connectionState: ConnectionState;
}
interface UseAgentSupervisorResult {
  supervisorResult: AgentSupervisorResult | null;
  supervisorError: string | null;
  startupGateRequired: boolean;
  startupEnsureState: StartupEnsureState;
  startupEnsureCompletedAt: number | null;
  startupEnsureError: string | null;
  restartAgent: () => void;
}
export function useAgentSupervisor({
  configIsLoaded,
  agentHost,
  agentPort,
  platformSupportsWsl,
  requiresWsl,
  autoStartEnabled,
  distroOverride,
  connectionState,
}: UseAgentSupervisorOptions): UseAgentSupervisorResult {
  const [supervisorResult, setSupervisorResult] = useState<AgentSupervisorResult | null>(null);
  const [supervisorError, setSupervisorError] = useState<string | null>(null);
  const [startupEnsureState, setStartupEnsureState] = useState<StartupEnsureState>("idle");
  const [startupEnsureCompletedAt, setStartupEnsureCompletedAt] = useState<number | null>(null);
  const [startupEnsureError, setStartupEnsureError] = useState<string | null>(null);
  const lastEnsureRef = useRef(0);
  const lastRequiresWslRef = useRef<boolean | null>(null);
  const startupGateKeyRef = useRef<string | null>(null);
  const startupRetryTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const ensureInFlightRef = useRef(false);
  const ensureQueuedRef = useRef(false);
  const latestInputsRef = useRef<SupervisorInputsSnapshot>({
    configIsLoaded,
    agentHost,
    agentPort,
    platformSupportsWsl,
    requiresWsl,
    autoStartEnabled,
    distroOverride,
  });
  const startupGateRequired =
    configIsLoaded &&
    autoStartEnabled &&
    isLoopbackHost(agentHost) &&
    platformSupportsWsl &&
    requiresWsl;
  const clearStartupRetryTimer = useCallback((): void => {
    if (!startupRetryTimerRef.current) return;
    clearTimeout(startupRetryTimerRef.current);
    startupRetryTimerRef.current = null;
  }, []);
  const ensureAgentRunning = useCallback((ignoreThrottle = false): void => {
    const inputs = latestInputsRef.current;
    if (!inputs.configIsLoaded || !inputs.autoStartEnabled) return;
    if (!isLoopbackHost(inputs.agentHost)) {
      setSupervisorError("Auto-start requires agentHost 127.0.0.1");
      return;
    }
    const now = Date.now();
    if (!ignoreThrottle && now - lastEnsureRef.current < ENSURE_THROTTLE_MS) return;
    lastEnsureRef.current = now;
    if (ensureInFlightRef.current) {
      ensureQueuedRef.current = true;
      return;
    }
    const requestConfig = buildRequestConfig(inputs);
    const startupGateActive = inputs.platformSupportsWsl && inputs.requiresWsl;
    const scheduleStartupRetry = (): void => {
      clearStartupRetryTimer();
      startupRetryTimerRef.current = setTimeout(() => {
        startupRetryTimerRef.current = null;
        ensureAgentRunning(true);
      }, ENSURE_THROTTLE_MS);
    };
    const markStartupGateReady = (): void => {
      clearStartupRetryTimer();
      setStartupEnsureState("ready");
      setStartupEnsureCompletedAt(Date.now());
      setStartupEnsureError(null);
    };
    const markStartupGateFailed = (message: string): void => {
      setStartupEnsureState("failed");
      setStartupEnsureError(message);
      scheduleStartupRetry();
    };
    ensureInFlightRef.current = true;
    if (startupGateActive) {
      setStartupEnsureState((prev) => (prev === "ready" ? prev : "ensuring"));
      setStartupEnsureError(null);
    }
    invoke<AgentSupervisorResult>("ensure_agent_running", { config: requestConfig })
      .then((result) => {
        lastRequiresWslRef.current = requestRequiresWsl(requestConfig);
        setSupervisorResult(result);
        setSupervisorError(result.message ?? null);
        if (!startupGateActive) return;
        if (result.status === "backoff") {
          markStartupGateFailed(result.message ?? "WSL backend launch backoff active");
          return;
        }
        markStartupGateReady();
      })
      .catch((err: unknown) => {
        const message = toSupervisorErrorMessage(err, "Agent start failed");
        setSupervisorError(message);
        if (!startupGateActive) return;
        markStartupGateFailed(message);
      })
      .finally(() => {
        ensureInFlightRef.current = false;
        if (!ensureQueuedRef.current) return;
        ensureQueuedRef.current = false;
        setTimeout(() => {
          ensureAgentRunning(true);
        }, 0);
      });
  }, [clearStartupRetryTimer]);
  useEffect(() => {
    latestInputsRef.current = {
      configIsLoaded,
      agentHost,
      agentPort,
      platformSupportsWsl,
      requiresWsl,
      autoStartEnabled,
      distroOverride,
    };
  }, [configIsLoaded, agentHost, agentPort, platformSupportsWsl, requiresWsl, autoStartEnabled, distroOverride]);
  useEffect(() => {
    const key = [
      configIsLoaded ? "1" : "0",
      autoStartEnabled ? "1" : "0",
      agentHost,
      agentPort.toString(),
      platformSupportsWsl ? "1" : "0",
      requiresWsl ? "1" : "0",
      distroOverride ?? "",
    ].join("|");
    if (startupGateKeyRef.current === key) return;
    startupGateKeyRef.current = key;
    clearStartupRetryTimer();
    setStartupEnsureState("idle");
    setStartupEnsureCompletedAt(null);
    setStartupEnsureError(null);
  }, [
    configIsLoaded,
    autoStartEnabled,
    agentHost,
    agentPort,
    platformSupportsWsl,
    requiresWsl,
    distroOverride,
    clearStartupRetryTimer,
  ]);
  useEffect(() => {
    if (!configIsLoaded) return;
    if (!autoStartEnabled) {
      setSupervisorResult(null);
      setSupervisorError(null);
      clearStartupRetryTimer();
      setStartupEnsureState("idle");
      setStartupEnsureCompletedAt(null);
      setStartupEnsureError(null);
      return;
    }
    if (!isLoopbackHost(agentHost)) {
      setSupervisorError("Auto-start requires agentHost 127.0.0.1");
      return;
    }
    const effectiveRequiresWsl = platformSupportsWsl && requiresWsl;
    const requiresWslChanged = lastRequiresWslRef.current !== effectiveRequiresWsl;
    if (connectionState.status === "connected" && !requiresWslChanged) return;
    ensureAgentRunning(requiresWslChanged);
  }, [
    configIsLoaded,
    autoStartEnabled,
    agentHost,
    agentPort,
    platformSupportsWsl,
    requiresWsl,
    distroOverride,
    connectionState.status,
    connectionState.reconnectAttempts,
    ensureAgentRunning,
    clearStartupRetryTimer,
  ]);
  useEffect(() => {
    if (!configIsLoaded || !autoStartEnabled) return;
    if (!platformSupportsWsl || !requiresWsl) return;
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
    platformSupportsWsl,
    requiresWsl,
    connectionState.status,
    ensureAgentRunning,
  ]);
  const restartAgent = useCallback((): void => {
    if (!configIsLoaded) return;
    const requestConfig: AgentSupervisorConfigPayload = platformSupportsWsl
      ? { port: agentPort, autoStart: true, wsl: { required: requiresWsl, distro: distroOverride } }
      : { port: agentPort, autoStart: true };
    invoke<AgentSupervisorResult>("restart_agent", { config: requestConfig })
      .then((result) => {
        setSupervisorResult(result);
        setSupervisorError(result.message ?? null);
        if (!startupGateRequired) return;
        if (result.status === "backoff") {
          setStartupEnsureState("failed");
          setStartupEnsureError(result.message ?? "WSL backend launch backoff active");
          clearStartupRetryTimer();
          startupRetryTimerRef.current = setTimeout(() => {
            startupRetryTimerRef.current = null;
            ensureAgentRunning(true);
          }, ENSURE_THROTTLE_MS);
          return;
        }
        clearStartupRetryTimer();
        setStartupEnsureState("ready");
        setStartupEnsureCompletedAt(Date.now());
        setStartupEnsureError(null);
      })
      .catch((err: unknown) => {
        const message = toSupervisorErrorMessage(err, "Agent restart failed");
        setSupervisorError(message);
        if (!startupGateRequired) return;
        setStartupEnsureState("failed");
        setStartupEnsureError(message);
        clearStartupRetryTimer();
        startupRetryTimerRef.current = setTimeout(() => {
          startupRetryTimerRef.current = null;
          ensureAgentRunning(true);
        }, ENSURE_THROTTLE_MS);
      });
  }, [
    agentPort,
    configIsLoaded,
    distroOverride,
    ensureAgentRunning,
    platformSupportsWsl,
    requiresWsl,
    startupGateRequired,
    clearStartupRetryTimer,
  ]);
  useEffect(() => {
    return () => {
      clearStartupRetryTimer();
    };
  }, [clearStartupRetryTimer]);
  return {
    supervisorResult,
    supervisorError,
    startupGateRequired,
    startupEnsureState,
    startupEnsureCompletedAt,
    startupEnsureError,
    restartAgent,
  };
}
