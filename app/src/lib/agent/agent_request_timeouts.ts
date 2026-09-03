// Path: app/src/lib/agent/agent_request_timeouts.ts
// Description: Per-command UI request timeout ladder (strictly above the agent and host->WSL budgets)

import type { SourceControlActionKind, UiCommand } from "../../shared/protocol.js";

const REQUEST_TIMEOUT_MS = 30_000;
const BUILD_BUNDLE_TIMEOUT_MS = 5 * 60_000;
// Source-control ladder. One request runs several bounded Git commands (20/60/120/180 s each,
// a status capture being five reads = 100 s), so each tier covers the request's end-to-end worst
// case: agent sum < host->WSL (120/280/340/380/420 s) < UI (+30 s). Discard is its own class in
// both ladders because it runs a pre-status, a restore, a reset, file removal, and a post-status.
// A UI timeout cancels nothing agent-side; the outcome is settled by reconciliation.
const SOURCE_CONTROL_READ_TIMEOUT_MS = 150_000;
const SOURCE_CONTROL_INDEX_TIMEOUT_MS = 310_000;
const SOURCE_CONTROL_DISCARD_TIMEOUT_MS = 370_000;
const SOURCE_CONTROL_COMMIT_TIMEOUT_MS = 410_000;
const SOURCE_CONTROL_REMOTE_TIMEOUT_MS = 450_000;

/** Also the reconciliation budget: how long an unknown outcome may be chased before settling. */
export function sourceControlActionTimeoutMs(kind: SourceControlActionKind): number {
  switch (kind) {
    case "stage":
    case "unstage":
      return SOURCE_CONTROL_INDEX_TIMEOUT_MS;
    case "discard":
      return SOURCE_CONTROL_DISCARD_TIMEOUT_MS;
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
