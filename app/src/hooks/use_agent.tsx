// Path: app/src/hooks/use_agent.tsx
// Description: Agent context provider and connection management hook

import {
  createContext,
  useContext,
  useEffect,
  useState,
  useCallback,
  useMemo,
  useRef,
  type ReactNode,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  createAgentClient,
  type AgentClient,
} from "../lib/agent/agent_client.js";
import { sendSetOptions } from "../lib/agent/messages.js";
import { useClientHello, type HelloState } from "./use_client_hello.js";
import { extractAppConfig, type AppConfig } from "../shared/config.js";
import {
  type ConnectionState,
  INITIAL_CONNECTION_STATE,
} from "../lib/agent/connection_state.js";
import type { AgentErrorEvent, AgentEvent } from "../shared/protocol.js";
import type { AppPaths } from "../types/app_paths.js";
import type { AgentSupervisorResult } from "../types/agent_supervisor.js";
import { useConfig } from "./use_config.js";
import { useAgentProbe } from "./agent/use_agent_probe.js";
import { useAgentShutdown } from "./agent/use_agent_shutdown.js";
import { useAgentSupervisor } from "./agent/use_agent_supervisor.js";

type EventHandler = (event: AgentEvent) => void;

interface AgentDiagnostics {
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

interface AgentContextValue {
  client: AgentClient | null;
  connectionState: ConnectionState;
  helloState: HelloState;
  agentError: AgentErrorEvent | null;
  agentDiagnostics: AgentDiagnostics | null;
  config: AppConfig;
  appPaths: AppPaths | null;
  autoStageOnChange: boolean;
  setAutoStageOnChange: (value: boolean) => void;
  restartAgent: () => void;
  subscribe: (handler: EventHandler) => () => void;
}

const AgentContext = createContext<AgentContextValue | null>(null);

interface AgentProviderProps {
  children: ReactNode;
}

export function AgentProvider({ children }: AgentProviderProps): React.JSX.Element {
  const { config: persistedConfig, isLoaded: configIsLoaded, setAutoStageGlobal } = useConfig();
  const config = useMemo(
    () => extractAppConfig(persistedConfig),
    [
      persistedConfig.agentHost,
      persistedConfig.agentPort,
      persistedConfig.autoStageGlobal,
      persistedConfig.repos,
      persistedConfig.recentFilesLimit,
    ]
  );

  const [connectionState, setConnectionState] = useState<ConnectionState>(
    INITIAL_CONNECTION_STATE
  );
  const [appPaths, setAppPaths] = useState<AppPaths | null>(null);
  const [autoStageOnChange, setAutoStageOnChangeState] = useState(config.autoStageGlobal);
  const [client, setClient] = useState<AgentClient | null>(null);
  const [agentError, setAgentError] = useState<AgentErrorEvent | null>(null);

  const handlersRef = useRef<Set<EventHandler>>(new Set());
  const outputWindowsRootRef = useRef(persistedConfig.outputWindowsRoot);
  const expectedHost = "127.0.0.1";
  const expectedUrl = `ws://${expectedHost}:${config.agentPort}`;
  const autoStartEnabled = persistedConfig.agentAutoStart;
  const distroOverride = persistedConfig.agentDistro;

  // clientHello lifecycle with reconnect support
  const helloState = useClientHello({
    client,
    connectionState,
    config,
    appPaths,
    autoStageOnChange,
  });

  const subscribe = useCallback((handler: EventHandler) => {
    handlersRef.current.add(handler);
    return () => {
      handlersRef.current.delete(handler);
    };
  }, []);

  const handleEvent = useCallback((event: AgentEvent) => {
    if (event.type === "error") {
      setAgentError(event);
    }
    for (const handler of handlersRef.current) {
      handler(event);
    }
  }, []);

  useEffect(() => {
    if (autoStageOnChange === config.autoStageGlobal) {
      return;
    }

    setAutoStageOnChangeState(config.autoStageGlobal);

    if (client && connectionState.status === "connected") {
      void sendSetOptions(client, config.autoStageGlobal).catch((err: unknown) => {
        console.error("[AgentProvider] setOptions failed:", err);
      });
    }
  }, [autoStageOnChange, client, config.autoStageGlobal, connectionState.status]);

  useEffect(() => {
    if (connectionState.status !== "connected" && agentError !== null) {
      setAgentError(null);
    }
  }, [agentError, connectionState.status]);
  const agentProbe = useAgentProbe({
    configIsLoaded,
    port: config.agentPort,
    connectionState,
  });
  const { supervisorResult, supervisorError, restartAgent } = useAgentSupervisor({
    configIsLoaded,
    agentHost: config.agentHost,
    agentPort: config.agentPort,
    autoStartEnabled,
    distroOverride,
    connectionState,
  });
  useAgentShutdown();

  // Initialize on mount (after config is loaded)
  useEffect(() => {
    // Don't initialize until config is loaded
    if (!configIsLoaded) return;

    let mounted = true;
    let agentClient: AgentClient | null = null;

    async function init(): Promise<void> {
      try {
        // Sync ref to the loaded config before resolving paths
        outputWindowsRootRef.current = persistedConfig.outputWindowsRoot;
        const outputRoot = outputWindowsRootRef.current;
        const paths = await invoke<AppPaths>("get_app_paths", {
          outputWindowsRoot: outputRoot,
        });
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
    // Note: outputWindowsRootRef is used instead of persistedConfig.outputWindowsRoot
    // to avoid reconnecting when only the output path changes. The refreshPaths effect
    // handles output path changes without triggering a full agent reconnection.
  }, [configIsLoaded, config.agentHost, config.agentPort, handleEvent]);

  // Refresh appPaths when outputWindowsRoot changes (after initial init)
  useEffect(() => {
    if (!configIsLoaded) return;
    // Skip if this is the first run or value hasn't changed
    if (persistedConfig.outputWindowsRoot === outputWindowsRootRef.current) return;

    async function refreshPaths(): Promise<void> {
      try {
        const paths = await invoke<AppPaths>("get_app_paths", {
          outputWindowsRoot: persistedConfig.outputWindowsRoot,
        });
        setAppPaths(paths);
        outputWindowsRootRef.current = persistedConfig.outputWindowsRoot;
        // The clientHello effect will re-send when appPaths changes
      } catch (err) {
        console.error("[AgentProvider] Failed to refresh paths:", err);
      }
    }

    void refreshPaths();
  }, [configIsLoaded, persistedConfig.outputWindowsRoot]);

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
    agentError,
    agentDiagnostics:
      connectionState.status !== "connected" &&
      (connectionState.reconnectAttempts > 0 ||
        connectionState.lastError ||
        supervisorError)
        ? {
            expectedUrl,
            probeListening: agentProbe?.listening ?? null,
            probeError: agentProbe?.error ?? null,
            lastError: connectionState.lastError,
            configuredHost: config.agentHost,
            port: config.agentPort,
            autoStartEnabled,
            supervisorStatus: supervisorResult?.status ?? null,
            supervisorError: supervisorError,
          }
        : null,
    config,
    appPaths,
    autoStageOnChange,
    setAutoStageOnChange,
    restartAgent,
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
