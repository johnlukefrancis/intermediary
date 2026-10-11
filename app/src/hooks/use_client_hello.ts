// Path: app/src/hooks/use_client_hello.ts
// Description: Owns the clientHello lifecycle: per-session runner, retry backoff, resync and WSL re-hello

import { useState, useEffect, useRef, useMemo, useCallback } from "react";
import type { AgentClient } from "../lib/agent/agent_client.js";
import type { ConnectionState } from "../lib/agent/connection_state.js";
import { sendClientHello } from "../lib/agent/messages.js";
import {
  ClientHelloRunner,
  type HelloOutcome,
  type HelloScheduler,
} from "../lib/agent/client_hello_runner.js";
import type { AppConfig } from "../shared/config.js";
import type { AppPaths } from "../types/app_paths.js";

type HelloStatus = "idle" | "pending" | "ok" | "error";

export interface HelloState {
  status: HelloStatus;
  watchedRepoIds: string[];
  /** Agent build reported by clientHelloResult; null until a hello succeeds and on disconnect */
  agentVersion: string | null;
  lastHelloAt: number | null;
  lastError: string | null;
}

export interface ClientHelloOwner {
  helloState: HelloState;
  /** Sends a hello through the session owner; false when disconnected or the hello failed. */
  resyncClientHello: () => Promise<boolean>;
  noteWslOnline: (generation: number) => void;
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
  agentVersion: null,
  lastHelloAt: null,
  lastError: null,
};

const PENDING_HELLO_STATE: HelloState = { ...INITIAL_HELLO_STATE, status: "pending" };

const scheduleTimeout: HelloScheduler = (run, delayMs) => {
  const handle = setTimeout(run, delayMs);
  return () => { clearTimeout(handle); };
};

function nextHelloState(prev: HelloState, outcome: HelloOutcome, now: number): HelloState {
  if (outcome.kind === "ok") {
    return {
      status: "ok",
      watchedRepoIds: outcome.result.watchedRepoIds,
      agentVersion: outcome.result.agentVersion,
      lastHelloAt: now,
      lastError: null,
    };
  }
  // A failed resync keeps the applied session usable; retries continue in the runner.
  if (prev.status === "ok") {
    return { ...prev, lastError: outcome.message };
  }
  return { ...INITIAL_HELLO_STATE, status: "error", lastError: outcome.message };
}

export function useClientHello(options: UseClientHelloOptions): ClientHelloOwner {
  const { client, connectionState, config, appPaths, autoStageOnChange } = options;
  const [helloState, setHelloState] = useState<HelloState>(INITIAL_HELLO_STATE);
  const runnerRef = useRef<ClientHelloRunner | null>(null);
  const autoStageRef = useRef(autoStageOnChange);
  const configRef = useRef(config);
  const appPathsRef = useRef(appPaths);
  const configKey = useMemo(() => JSON.stringify(config), [config]);
  const hasAppPaths = appPaths !== null;
  const stagingHostRoot = appPaths?.stagingHostRoot ?? null;
  const stagingWslRoot = appPaths?.stagingWslRoot ?? null;

  useEffect(() => {
    autoStageRef.current = autoStageOnChange;
    configRef.current = config;
    appPathsRef.current = appPaths;
  }, [appPaths, autoStageOnChange, config]);

  useEffect(() => {
    if (connectionState.status !== "connected") {
      setHelloState((prev) => (prev.status === "idle" ? prev : INITIAL_HELLO_STATE));
      return;
    }
    const paths = appPathsRef.current;
    if (!client || !hasAppPaths || !paths) {
      return;
    }

    // One runner per connection + hello inputs; disposal drops its retries and late results.
    const sessionConfig = configRef.current;
    const runner = new ClientHelloRunner({
      send: () => sendClientHello(
        client,
        sessionConfig,
        paths.stagingHostRoot,
        paths.stagingWslRoot,
        autoStageRef.current
      ),
      onStart: () => {
        setHelloState((prev) => (prev.status === "ok" ? prev : { ...prev, status: "pending" }));
      },
      onOutcome: (outcome) => {
        if (outcome.kind === "error") {
          console.error("[useClientHello] clientHello failed:", outcome.message);
        }
        setHelloState((prev) => nextHelloState(prev, outcome, Date.now()));
      },
      schedule: scheduleTimeout,
    });
    runnerRef.current = runner;
    setHelloState(PENDING_HELLO_STATE);
    void runner.request();

    return () => {
      runner.dispose();
      if (runnerRef.current === runner) {
        runnerRef.current = null;
      }
    };
  }, [
    client,
    configKey,
    connectionState.connectedAt,
    connectionState.status,
    hasAppPaths,
    stagingHostRoot,
    stagingWslRoot,
  ]);

  const resyncClientHello = useCallback(
    (): Promise<boolean> => runnerRef.current?.request() ?? Promise.resolve(false),
    []
  );
  const noteWslOnline = useCallback((generation: number): void => {
    runnerRef.current?.noteWslOnline(generation);
  }, []);

  return { helloState, resyncClientHello, noteWslOnline };
}
