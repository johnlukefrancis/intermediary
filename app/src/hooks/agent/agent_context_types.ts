// Path: app/src/hooks/agent/agent_context_types.ts
// Description: Shared context and event handler types for the agent provider hook

import type { AgentClient } from "../../lib/agent/agent_client.js";
import type { ConnectionState } from "../../lib/agent/connection_state.js";
import type { HelloState } from "../use_client_hello.js";
import type { AppConfig } from "../../shared/config.js";
import type { AgentErrorEvent, AgentEvent } from "../../shared/protocol.js";
import type { AppPaths } from "../../types/app_paths.js";
import type { AgentDiagnostics } from "./agent_diagnostics.js";

export type EventHandler = (event: AgentEvent) => void;

export interface AgentContextValue {
  client: AgentClient | null;
  connectionState: ConnectionState;
  helloState: HelloState;
  rehydrateEpoch: number;
  agentError: AgentErrorEvent | null;
  agentDiagnostics: AgentDiagnostics | null;
  platformSupportsWsl: boolean;
  config: AppConfig;
  appPaths: AppPaths | null;
  autoStageOnChange: boolean;
  setAutoStageOnChange: (value: boolean) => void;
  resyncClientHello: () => Promise<boolean>;
  restartAgent: () => void;
  subscribe: (handler: EventHandler) => () => void;
}
