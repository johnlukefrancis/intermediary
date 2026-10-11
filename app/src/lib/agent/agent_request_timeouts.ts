// Path: app/src/lib/agent/agent_request_timeouts.ts
// Description: Per-command UI request timeout ladder (strictly above the agent and host->WSL budgets)

import type { SourceControlActionKind, UiCommand } from "../../shared/protocol.js";

const REQUEST_TIMEOUT_MS = 30_000;
const BUILD_BUNDLE_TIMEOUT_MS = 5 * 60_000;
// Source-control budgets cover sequential Git operations plus 30 s above host forwarding.
// Mutations reconcile unknown outcomes; only read-only inspections use request cancellation.
const SOURCE_CONTROL_READ_TIMEOUT_MS = 150_000;
const SOURCE_CONTROL_INDEX_TIMEOUT_MS = 310_000;
const SOURCE_CONTROL_DISCARD_TIMEOUT_MS = 370_000;
const SOURCE_CONTROL_COMMIT_TIMEOUT_MS = 410_000;
const SOURCE_CONTROL_REMOTE_TIMEOUT_MS = 450_000;
// A directory import runs a bounded recursive copy; budget generously above the agent's own cap.
const IMPORT_FILES_TIMEOUT_MS = 330_000;
// Delete/move/rename touch the worktree only (no recursive host copy); copy reuses the import budget.
const WORKTREE_ENTRY_TIMEOUT_MS = 90_000;

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

/** Mutation timeouts abandon the promise; inspection timeouts also cancel the remote scan. */
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
    case "importFiles":
      return IMPORT_FILES_TIMEOUT_MS;
    case "worktreeAction":
      return command.action.kind === "copy" ? IMPORT_FILES_TIMEOUT_MS : WORKTREE_ENTRY_TIMEOUT_MS;
    default:
      return REQUEST_TIMEOUT_MS;
  }
}
