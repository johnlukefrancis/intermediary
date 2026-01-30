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
import { sendClientHello } from "../lib/agent/messages.js";
import { getDefaultConfig, type AppConfig } from "../shared/config.js";
import {
  type ConnectionState,
  INITIAL_CONNECTION_STATE,
} from "../lib/agent/connection_state.js";
import type { AgentEvent } from "../shared/protocol.js";
import type { AppPaths } from "../types/app_paths.js";

type EventHandler = (event: AgentEvent) => void;

interface AgentContextValue {
  client: AgentClient | null;
  connectionState: ConnectionState;
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
  const [connectionState, setConnectionState] = useState<ConnectionState>(
    INITIAL_CONNECTION_STATE
  );
  const [appPaths, setAppPaths] = useState<AppPaths | null>(null);
  const [config] = useState<AppConfig>(() => getDefaultConfig());
  const [autoStageOnChange, setAutoStageOnChange] = useState(config.autoStageGlobal);
  const [client, setClient] = useState<AgentClient | null>(null);

  const handlersRef = useRef<Set<EventHandler>>(new Set());

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
    if (
      connectionState.status !== "connected" ||
      !client ||
      !appPaths
    ) {
      return;
    }

    void sendClientHello(
      client,
      config,
      appPaths.stagingWslRoot,
      appPaths.stagingWindowsRoot,
      autoStageOnChange
    ).catch((err: unknown) => {
      console.error("[AgentProvider] clientHello failed:", err);
    });
  }, [connectionState.status, client, appPaths, config, autoStageOnChange]);

  const value: AgentContextValue = {
    client,
    connectionState,
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
