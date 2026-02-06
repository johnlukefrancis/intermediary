// Path: app/src/types/agent_supervisor.ts
// Description: Types for Tauri dual-agent supervisor responses

export type AgentSupervisorStatus =
  | "started"
  | "already_running"
  | "disabled"
  | "backoff";

export interface AgentSupervisorResult {
  status: AgentSupervisorStatus;
  port: number;
  wslPort: number;
  requiresWsl: boolean;
  agentDir: string;
  logDir: string;
  message?: string | null;
}
