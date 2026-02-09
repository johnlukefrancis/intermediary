// Path: app/src/lib/agent/error_codes.ts
// Description: Parse backend response error codes from agent_client error messages

/**
 * Extracts protocol error code from `agent_client` error messages like:
 * "NOT_CONFIGURED: Staging not configured".
 */
export function parseAgentErrorCode(error: unknown): string | null {
  const message = error instanceof Error ? error.message : String(error);
  const sep = message.indexOf(":");
  if (sep <= 0) {
    return null;
  }
  const code = message.slice(0, sep).trim();
  if (!/^[A-Z0-9_]+$/.test(code)) {
    return null;
  }
  return code;
}

export function isStagingNotConfiguredError(error: unknown): boolean {
  const code = parseAgentErrorCode(error);
  return code === "NOT_CONFIGURED" || code === "MISSING_WSL_ROOT";
}
