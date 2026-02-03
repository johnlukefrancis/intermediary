// Path: app/src/hooks/agent/use_agent_probe.ts
// Description: Probe the agent port when disconnected for diagnostics

import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ConnectionState } from "../../lib/agent/connection_state.js";

export interface AgentPortProbeResult {
  listening: boolean;
  error?: string | null;
}

interface UseAgentProbeOptions {
  configIsLoaded: boolean;
  port: number;
  connectionState: ConnectionState;
}

export function useAgentProbe({
  configIsLoaded,
  port,
  connectionState,
}: UseAgentProbeOptions): AgentPortProbeResult | null {
  const [probe, setProbe] = useState<AgentPortProbeResult | null>(null);
  const lastProbeKeyRef = useRef<string | null>(null);

  useEffect(() => {
    if (!configIsLoaded) return;

    if (connectionState.status === "connected") {
      setProbe(null);
      lastProbeKeyRef.current = null;
      return;
    }

    const attempts = connectionState.reconnectAttempts;
    const shouldProbe = attempts >= 1 && (attempts === 1 || attempts % 3 === 0);
    if (!shouldProbe) return;

    const key = `${attempts}:${port}:${connectionState.lastError ?? ""}`;
    if (key === lastProbeKeyRef.current) return;
    lastProbeKeyRef.current = key;

    invoke<AgentPortProbeResult>("probe_agent_port", { port })
      .then((result) => {
        setProbe({
          listening: result.listening,
          error: result.error ?? null,
        });
      })
      .catch((err: unknown) => {
        const message = err instanceof Error ? err.message : "Probe failed";
        setProbe({ listening: false, error: message });
      });
  }, [configIsLoaded, connectionState, port]);

  return probe;
}
