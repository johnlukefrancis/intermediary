// Path: app/src/hooks/agent/use_agent_supervisor_helpers.ts
// Description: Shared parsing and request helpers for agent supervisor hook

interface AgentSupervisorWslConfigPayload {
  required: boolean;
  distro: string | null;
}

export interface AgentSupervisorConfigPayload {
  port: number;
  autoStart: boolean;
  wsl?: AgentSupervisorWslConfigPayload;
}

export interface SupervisorInputsSnapshot {
  configIsLoaded: boolean;
  agentHost: string;
  agentPort: number;
  platformSupportsWsl: boolean;
  requiresWsl: boolean;
  autoStartEnabled: boolean;
  distroOverride: string | null;
}

export function toSupervisorErrorMessage(err: unknown, fallback: string): string {
  if (typeof err === "string" && err.trim().length > 0) {
    return err;
  }
  if (err instanceof Error && err.message.trim().length > 0) {
    return err.message;
  }
  if (typeof err === "object" && err !== null && "message" in err) {
    const maybeMessage = (err as { message?: unknown }).message;
    if (typeof maybeMessage === "string" && maybeMessage.trim().length > 0) {
      return maybeMessage;
    }
  }
  return fallback;
}

export function buildRequestConfig(
  inputs: SupervisorInputsSnapshot
): AgentSupervisorConfigPayload {
  if (!inputs.platformSupportsWsl) {
    return {
      port: inputs.agentPort,
      autoStart: inputs.autoStartEnabled,
    };
  }

  return {
    port: inputs.agentPort,
    autoStart: inputs.autoStartEnabled,
    wsl: {
      required: inputs.requiresWsl,
      distro: inputs.distroOverride,
    },
  };
}

export function requestRequiresWsl(config: AgentSupervisorConfigPayload): boolean {
  return config.wsl?.required ?? false;
}

export function isLoopbackHost(host: string): boolean {
  return host === "127.0.0.1" || host === "localhost";
}
