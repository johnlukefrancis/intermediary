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
import { sendClientHello, sendSetOptions } from "../lib/agent/messages.js";
import { useClientHello } from "./use_client_hello.js";
import { useResumeDetector } from "./use_resume_detector.js";
import { extractAppConfig } from "../shared/config.js";
import {
  type ConnectionState,
  INITIAL_CONNECTION_STATE,
} from "../lib/agent/connection_state.js";
import type { AgentErrorEvent, AgentEvent } from "../shared/protocol.js";
import type { AppPaths } from "../types/app_paths.js";
import { useConfig } from "./use_config.js";
import {
  buildAgentDiagnostics,
  hostSupportsWsl,
} from "./agent/agent_diagnostics.js";
import {
  type AgentContextValue,
  type EventHandler,
} from "./agent/agent_context_types.js";
import { useAgentProbe } from "./agent/use_agent_probe.js";
import { useAgentShutdown } from "./agent/use_agent_shutdown.js";
import { useAgentSupervisor } from "./agent/use_agent_supervisor.js";
import { nextAgentError } from "./agent/wsl_transport_errors.js";

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
      persistedConfig.classificationExcludes,
    ]
  );

  const [connectionState, setConnectionState] = useState<ConnectionState>(
    INITIAL_CONNECTION_STATE
  );
  const [appPaths, setAppPaths] = useState<AppPaths | null>(null);
  const [autoStageOnChange, setAutoStageOnChangeState] = useState(config.autoStageGlobal);
  const [client, setClient] = useState<AgentClient | null>(null);
  const [rehydrateEpoch, setRehydrateEpoch] = useState(0);
  const [agentError, setAgentError] = useState<AgentErrorEvent | null>(null);
  const helloSyncInFlightRef = useRef<Promise<boolean> | null>(null);

  const handlersRef = useRef<Set<EventHandler>>(new Set());
  const outputHostRootRef = useRef(persistedConfig.outputWindowsRoot);
  const expectedHost = "127.0.0.1";
  const expectedUrl = `ws://${expectedHost}:${config.agentPort}`;
  const autoStartEnabled = persistedConfig.agentAutoStart;
  const distroOverride = persistedConfig.agentDistro;
  const requiresWsl = config.repos.some((repo) => repo.root.kind === "wsl");
  const platformSupportsWsl = hostSupportsWsl();
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
    if (event.type === "error" || event.type === "wslBackendStatus") {
      setAgentError((current) => nextAgentError(current, event));
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
  const {
    supervisorResult,
    supervisorError,
    startupGateRequired,
    startupEnsureState,
    startupEnsureError,
    restartAgent,
  } = useAgentSupervisor({
    configIsLoaded,
    agentHost: config.agentHost,
    agentPort: config.agentPort,
    platformSupportsWsl,
    requiresWsl,
    autoStartEnabled,
    distroOverride,
    connectionState,
  });
  useAgentShutdown();
  const startupGateReady = !startupGateRequired || startupEnsureState === "ready";
  const triggerResumeRecovery = useCallback((): void => {
    setRehydrateEpoch((previous) => previous + 1);

    if (!client) {
      return;
    }

    if (
      connectionState.status === "connected" ||
      connectionState.status === "connecting" ||
      connectionState.status === "reconnecting"
    ) {
      client.disconnect();
      client.connect();
      return;
    }

    client.connect();
  }, [client, connectionState.status]);
  useResumeDetector({
    enabled: configIsLoaded && startupGateReady,
    onResume: triggerResumeRecovery,
  });

  useEffect(() => {
    if (!configIsLoaded || !startupGateReady) return;

    let mounted = true;
    let agentClient: AgentClient | null = null;

    async function init(): Promise<void> {
      try {
        outputHostRootRef.current = persistedConfig.outputWindowsRoot;
        const outputRoot = outputHostRootRef.current;
        const paths = await invoke<AppPaths>("get_app_paths", {
          outputHostRoot: outputRoot,
        });
        if (!mounted) return;
        setAppPaths(paths);

        agentClient = createAgentClient({
          host: config.agentHost,
          port: config.agentPort,
          authToken: paths.agentWsToken,
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
  }, [configIsLoaded, startupGateReady, config.agentHost, config.agentPort, handleEvent]);
  useEffect(() => {
    if (!configIsLoaded) return;
    if (persistedConfig.outputWindowsRoot === outputHostRootRef.current) return;

    async function refreshPaths(): Promise<void> {
      try {
        const paths = await invoke<AppPaths>("get_app_paths", {
          outputHostRoot: persistedConfig.outputWindowsRoot,
        });
        setAppPaths(paths);
        outputHostRootRef.current = persistedConfig.outputWindowsRoot;
      } catch (err) {
        console.error("[AgentProvider] Failed to refresh paths:", err);
      }
    }

    void refreshPaths();
  }, [configIsLoaded, persistedConfig.outputWindowsRoot]);

  const setAutoStageOnChange = useCallback(
    (value: boolean) => {
      setAutoStageOnChangeState(value);
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

  const resyncClientHello = useCallback(async (): Promise<boolean> => {
    if (!client || !appPaths || connectionState.status !== "connected") {
      return false;
    }

    if (helloSyncInFlightRef.current) {
      return helloSyncInFlightRef.current;
    }

    const syncPromise = sendClientHello(
      client,
      config,
      appPaths.stagingHostRoot,
      appPaths.stagingWslRoot,
      autoStageOnChange
    )
      .then(() => {
        return true;
      })
      .catch((err: unknown) => {
        console.error("[AgentProvider] clientHello re-sync failed:", err);
        return false;
      })
      .finally(() => {
        helloSyncInFlightRef.current = null;
      });

    helloSyncInFlightRef.current = syncPromise;
    return syncPromise;
  }, [appPaths, autoStageOnChange, client, config, connectionState.status]);

  const agentDiagnostics = buildAgentDiagnostics({
    connectionState,
    expectedUrl,
    probeListening: agentProbe?.listening ?? null,
    probeError: agentProbe?.error ?? null,
    configuredHost: config.agentHost,
    port: config.agentPort,
    autoStartEnabled,
    supervisorStatus: supervisorResult?.status ?? null,
    supervisorError,
    startupGateRequired,
    startupEnsureState,
    startupEnsureError,
  });

  const value: AgentContextValue = {
    client,
    connectionState,
    helloState,
    rehydrateEpoch,
    agentError,
    agentDiagnostics,
    platformSupportsWsl,
    config,
    appPaths,
    autoStageOnChange,
    setAutoStageOnChange,
    resyncClientHello,
    restartAgent,
    subscribe,
  };

  return (
    <AgentContext.Provider value={value}>{children}</AgentContext.Provider>
  );
}

export function useAgent(): AgentContextValue {
  const context = useContext(AgentContext);
  if (!context) {
    throw new Error("useAgent must be used within AgentProvider");
  }
  return context;
}
