// Path: app/src/hooks/source_control/source_control_failures.ts
// Description: Classify agent rejections for source-control reads and actions

import { parseAgentErrorCode } from "../../lib/agent/error_codes.js";

/** Codes that mean the action definitively did not run or failed; Git's own text is shown */
const DEFINITIVE_ACTION_CODES = new Set([
  "INVALID_PATH",
  "INVALID_COMMIT_MESSAGE",
  "INVALID_REPO",
]);

export type ActionFailure =
  | { type: "agentUpdateRequired"; message: string }
  | { type: "rejected"; code: string; message: string }
  /** No GIT_* code: transport failure, UI timeout, closed socket. The action may have landed. */
  | { type: "unknownOutcome" };

/** Text after "<CODE>: " when a code is present, otherwise the whole message */
export function agentErrorMessage(error: unknown): string {
  const message = error instanceof Error ? error.message : String(error);
  const code = parseAgentErrorCode(error);
  if (code === null) return message;
  return message.slice(code.length + 1).trim();
}

export function classifyActionFailure(error: unknown): ActionFailure {
  const code = parseAgentErrorCode(error);
  if (code === "UNKNOWN_COMMAND") {
    return { type: "agentUpdateRequired", message: agentErrorMessage(error) };
  }
  if (code !== null && (code.startsWith("GIT_") || DEFINITIVE_ACTION_CODES.has(code))) {
    return { type: "rejected", code, message: agentErrorMessage(error) };
  }
  return { type: "unknownOutcome" };
}
