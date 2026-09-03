// Path: app/src/lib/agent/agent_request_timeouts.ts
// Description: Per-command UI request timeout ladder (strictly above the agent and host->WSL budgets)

import type { SourceControlActionKind, UiCommand } from "../../shared/protocol.js";

const REQUEST_TIMEOUT_MS = 30_000;
const BUILD_BUNDLE_TIMEOUT_MS = 5 * 60_000;
// Source-control ladder: one request runs several bounded Git commands (each 20/60/120/180 s),
// the host->WSL forward budget is 90/120/240/300 s, and the UI sits strictly above it.
const SOURCE_CONTROL_READ_TIMEOUT_MS = 120_000;
const SOURCE_CONTROL_INDEX_TIMEOUT_MS = 150_000;
const SOURCE_CONTROL_COMMIT_TIMEOUT_MS = 300_000;
const SOURCE_CONTROL_REMOTE_TIMEOUT_MS = 360_000;

function sourceControlActionTimeoutMs(kind: SourceControlActionKind): number {
  switch (kind) {
    case "stage":
    case "unstage":
    case "discard":
      return SOURCE_CONTROL_INDEX_TIMEOUT_MS;
    case "commit":
      return SOURCE_CONTROL_COMMIT_TIMEOUT_MS;
    case "push":
    case "pull":
      return SOURCE_CONTROL_REMOTE_TIMEOUT_MS;
  }
}

/** A UI timeout only abandons the pending promise; it cancels nothing agent-side. */
export function getRequestTimeoutMs(command: UiCommand): number {
  switch (command.type) {
    case "buildBundle":
      return BUILD_BUNDLE_TIMEOUT_MS;
    case "sourceControlStatus":
    case "sourceControlDiff":
    case "sourceControlImageDiff":
      return SOURCE_CONTROL_READ_TIMEOUT_MS;
    case "sourceControlAction":
      return sourceControlActionTimeoutMs(command.action.kind);
    default:
      return REQUEST_TIMEOUT_MS;
  }
}
