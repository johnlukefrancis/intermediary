// Path: app/src/types/agent_supervisor.ts
// Description: Types for Tauri host-agent supervisor responses

export type AgentSupervisorStatus =
  | "started"
  | "already_running"
  | "disabled"
  | "backoff";

export interface AgentSupervisorWslStatus {
  required: boolean;
  port: number;
}

export interface AgentSupervisorResult {
  status: AgentSupervisorStatus;
  port: number;
  supportsWsl: boolean;
  wsl?: AgentSupervisorWslStatus;
  agentDir: string;
  logDir: string;
  message?: string | null;
}
