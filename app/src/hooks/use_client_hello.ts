// Path: app/src/hooks/use_client_hello.ts
// Description: Custom hook for clientHello lifecycle with reconnect support

import { useState, useEffect, useRef, useMemo } from "react";
import type { AgentClient } from "../lib/agent/agent_client.js";
import type { ConnectionState } from "../lib/agent/connection_state.js";
import { sendClientHello } from "../lib/agent/messages.js";
import type { AppConfig } from "../shared/config.js";
import type { AppPaths } from "../types/app_paths.js";

type HelloStatus = "idle" | "pending" | "ok" | "error";

export interface HelloState {
  status: HelloStatus;
  watchedRepoIds: string[];
  lastHelloAt: number | null;
  lastError: string | null;
}

interface UseClientHelloOptions {
  client: AgentClient | null;
  connectionState: ConnectionState;
  config: AppConfig;
  appPaths: AppPaths | null;
  autoStageOnChange: boolean;
}

const INITIAL_HELLO_STATE: HelloState = {
  status: "idle",
  watchedRepoIds: [],
  lastHelloAt: null,
  lastError: null,
};

function buildConfigKey(config: AppConfig): string {
  return JSON.stringify(config);
}

export function useClientHello(options: UseClientHelloOptions): HelloState {
  const { client, connectionState, config, appPaths, autoStageOnChange } = options;
  const [helloState, setHelloState] = useState<HelloState>(INITIAL_HELLO_STATE);

  // Stable ref for autoStageOnChange to avoid effect churn
  const autoStageRef = useRef(autoStageOnChange);
  const lastHelloKeyRef = useRef<string | null>(null);
  const configKey = useMemo(() => buildConfigKey(config), [config]);

  useEffect(() => {
    autoStageRef.current = autoStageOnChange;
  }, [autoStageOnChange]);

  useEffect(() => {
    // Reset to idle when not connected
    if (connectionState.status !== "connected") {
      lastHelloKeyRef.current = null;
      setHelloState((prev) =>
        prev.status === "idle" ? prev : INITIAL_HELLO_STATE
      );
      return;
    }

    if (!client || !appPaths) {
      return;
    }

    const helloKey = [
      connectionState.connectedAt ?? "no-conn",
      appPaths.stagingHostRoot,
      appPaths.stagingWslRoot ?? "no-wsl-root",
      configKey,
    ].join("|");

    if (lastHelloKeyRef.current === helloKey) {
      return;
    }
    lastHelloKeyRef.current = helloKey;

    // Send clientHello on (re)connect and when hello inputs change while connected
    let cancelled = false;
    setHelloState({
      status: "pending",
      watchedRepoIds: [],
      lastHelloAt: null,
      lastError: null,
    });

    void sendClientHello(
      client,
      config,
      appPaths.stagingHostRoot,
      appPaths.stagingWslRoot,
      autoStageRef.current
    )
      .then((result) => {
        if (cancelled) return;
        setHelloState({
          status: "ok",
          watchedRepoIds: result.watchedRepoIds,
          lastHelloAt: Date.now(),
          lastError: null,
        });
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        const message = err instanceof Error ? err.message : "clientHello failed";
        console.error("[useClientHello] clientHello failed:", err);
        setHelloState({
          status: "error",
          watchedRepoIds: [],
          lastHelloAt: null,
          lastError: message,
        });
      });

    return () => {
      cancelled = true;
    };
  }, [
    appPaths,
    client,
    config,
    configKey,
    connectionState.connectedAt,
    connectionState.status,
  ]);

  return helloState;
}
