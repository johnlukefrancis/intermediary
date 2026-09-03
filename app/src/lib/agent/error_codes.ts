// Path: app/src/lib/agent/error_codes.ts
// Description: Typed agent response error plus accessors for its code, message, and details

/**
 * A protocol error response from the agent. `agent_client` rejects with this so callers read the
 * code and the structured `details` payload instead of re-parsing the message text.
 */
export class AgentResponseError extends Error {
  readonly code: string;
  readonly serverMessage: string;
  // TODO(ts-precision): protocol `details` is an open per-code payload; each reader validates it.
  readonly details: unknown;

  constructor(code: string, serverMessage: string, details: unknown) {
    super(`${code}: ${serverMessage}`);
    this.name = "AgentResponseError";
    this.code = code;
    this.serverMessage = serverMessage;
    this.details = details;
  }
}

/** The agent's error code, or null for a transport failure (no response was ever parsed). */
export function parseAgentErrorCode(error: unknown): string | null {
  return error instanceof AgentResponseError ? error.code : null;
}

/** The structured payload the agent attached to this error, or null for a transport failure. */
export function agentErrorDetails(error: unknown): unknown {
  return error instanceof AgentResponseError ? error.details : null;
}

export function isStagingNotConfiguredError(error: unknown): boolean {
  const code = parseAgentErrorCode(error);
  return code === "NOT_CONFIGURED" || code === "MISSING_WSL_ROOT";
}
