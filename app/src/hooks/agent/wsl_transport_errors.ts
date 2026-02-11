// Path: app/src/hooks/agent/wsl_transport_errors.ts
// Description: Classifies WSL transport errors and clears stale errors on explicit backend recovery events

import type {
  AgentErrorEvent,
  AgentEvent,
  WslBackendStatusEvent,
} from "../../shared/protocol.js";

const WSL_TRANSPORT_CODES = new Set([
  "WSL_BACKEND_UNAVAILABLE",
  "WSL_BACKEND_TIMEOUT",
]);

export function isWslTransportError(error: AgentErrorEvent | null): boolean {
  if (!error || error.scope !== "wslBackend") {
    return false;
  }

  const rawCode = error.details?.rawCode;
  return typeof rawCode === "string" && WSL_TRANSPORT_CODES.has(rawCode);
}

function shouldClearWslTransportError(
  current: AgentErrorEvent | null,
  status: WslBackendStatusEvent
): boolean {
  return status.status === "online" && isWslTransportError(current);
}

export function nextAgentError(
  current: AgentErrorEvent | null,
  event: AgentEvent
): AgentErrorEvent | null {
  if (event.type === "error") {
    return event;
  }

  if (event.type === "wslBackendStatus" && shouldClearWslTransportError(current, event)) {
    return null;
  }

  return current;
}
