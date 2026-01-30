// Path: app/src/hooks/use_agent.tsx
// Description: Agent context provider and connection management hook

import {
  createContext,
  useContext,
  useEffect,
  useState,
  useCallback,
  useRef,
  type ReactNode,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  createAgentClient,
  type AgentClient,
} from "../lib/agent/agent_client.js";
import { sendClientHello, sendSetOptions } from "../lib/agent/messages.js";
import { extractAppConfig, type AppConfig } from "../shared/config.js";
import {
  type ConnectionState,
  INITIAL_CONNECTION_STATE,
} from "../lib/agent/connection_state.js";
import type { AgentEvent } from "../shared/protocol.js";
import type { AppPaths } from "../types/app_paths.js";
import { useConfig } from "./use_config.js";

type EventHandler = (event: AgentEvent) => void;

type HelloStatus = "idle" | "pending" | "ok" | "error";

interface HelloState {
  status: HelloStatus;
  watchedRepoIds: string[];
  lastHelloAt: number | null;
  lastError: string | null;
}

interface AgentContextValue {
  client: AgentClient | null;
  connectionState: ConnectionState;
  helloState: HelloState;
  config: AppConfig;
  appPaths: AppPaths | null;
  autoStageOnChange: boolean;
  setAutoStageOnChange: (value: boolean) => void;
  subscribe: (handler: EventHandler) => () => void;
}

const AgentContext = createContext<AgentContextValue | null>(null);

interface AgentProviderProps {
  children: ReactNode;
}

export function AgentProvider({ children }: AgentProviderProps): React.JSX.Element {
  const { config: persistedConfig, setAutoStageGlobal } = useConfig();
  const config = extractAppConfig(persistedConfig);

  const [connectionState, setConnectionState] = useState<ConnectionState>(
    INITIAL_CONNECTION_STATE
  );
  const [appPaths, setAppPaths] = useState<AppPaths | null>(null);
  const [autoStageOnChange, setAutoStageOnChangeState] = useState(config.autoStageGlobal);
  const [client, setClient] = useState<AgentClient | null>(null);
  const [helloState, setHelloState] = useState<HelloState>({
    status: "idle",
    watchedRepoIds: [],
    lastHelloAt: null,
    lastError: null,
  });

  const handlersRef = useRef<Set<EventHandler>>(new Set());
  const autoStageRef = useRef(autoStageOnChange);

  const subscribe = useCallback((handler: EventHandler) => {
    handlersRef.current.add(handler);
    return () => {
      handlersRef.current.delete(handler);
    };
  }, []);

  const handleEvent = useCallback((event: AgentEvent) => {
    for (const handler of handlersRef.current) {
      handler(event);
    }
  }, []);

  useEffect(() => {
    autoStageRef.current = autoStageOnChange;
  }, [autoStageOnChange]);

  useEffect(() => {
    if (autoStageOnChange === config.autoStageGlobal) {
      return;
    }

    setAutoStageOnChangeState(config.autoStageGlobal);
    autoStageRef.current = config.autoStageGlobal;

    if (client && connectionState.status === "connected") {
      void sendSetOptions(client, config.autoStageGlobal).catch((err: unknown) => {
        console.error("[AgentProvider] setOptions failed:", err);
      });
    }
  }, [autoStageOnChange, client, config.autoStageGlobal, connectionState.status]);

  // Initialize on mount
  useEffect(() => {
    let mounted = true;
    let agentClient: AgentClient | null = null;

    async function init(): Promise<void> {
      try {
        const paths = await invoke<AppPaths>("get_app_paths");
        if (!mounted) return;
        setAppPaths(paths);

        agentClient = createAgentClient({
          host: config.agentHost,
          port: config.agentPort,
          onConnectionChange: (state) => {
            if (mounted) setConnectionState(state);
          },
          onEvent: handleEvent,
        });

        setClient(agentClient);
        agentClient.connect();
      } catch (err) {
        console.error("[AgentProvider] Init failed:", err);
      }
    }

    void init();

    return () => {
      mounted = false;
      if (agentClient) {
        agentClient.disconnect();
      }
    };
  }, [config.agentHost, config.agentPort, handleEvent]);

  // Send clientHello when connected
  useEffect(() => {
    if (connectionState.status !== "connected") {
      setHelloState((prev) =>
        prev.status === "idle"
          ? prev
          : {
              status: "idle",
              watchedRepoIds: [],
              lastHelloAt: null,
              lastError: null,
            }
      );
      return;
    }

    if (!client || !appPaths) {
      return;
    }

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
      appPaths.stagingWslRoot,
      appPaths.stagingWindowsRoot,
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
        console.error("[AgentProvider] clientHello failed:", err);
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
  }, [connectionState.status, client, appPaths, config]);

  const setAutoStageOnChange = useCallback(
    (value: boolean) => {
      setAutoStageOnChangeState(value);
      // Persist to config
      setAutoStageGlobal(value);

      if (!client || connectionState.status !== "connected") {
        return;
      }
      void sendSetOptions(client, value)
        .then((result) => {
          setAutoStageOnChangeState(result.autoStageOnChange);
        })
        .catch((err: unknown) => {
          console.error("[AgentProvider] setOptions failed:", err);
        });
    },
    [client, connectionState.status, setAutoStageGlobal]
  );

  const value: AgentContextValue = {
    client,
    connectionState,
    helloState,
    config,
    appPaths,
    autoStageOnChange,
    setAutoStageOnChange,
    subscribe,
  };

  return (
    <AgentContext.Provider value={value}>
      {children}
    </AgentContext.Provider>
  );
}

export function useAgent(): AgentContextValue {
  const context = useContext(AgentContext);
  if (!context) {
    throw new Error("useAgent must be used within AgentProvider");
  }
  return context;
}
