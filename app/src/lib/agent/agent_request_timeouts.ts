// Path: app/src/lib/agent/agent_request_timeouts.ts
// Description: Per-command UI request timeout ladder (strictly above the agent and host->WSL budgets)

import type { SourceControlActionKind, UiCommand, WorktreeAction } from "../../shared/protocol.js";

// Each tier is the host->WSL forward budget (im_host_agent wsl_backend_client/timeouts.rs) + 30 s,
// so the host always answers or times out first; agent_request_timeouts_test.ts pins the relation.
type FlatCommandType = Exclude<UiCommand["type"], "sourceControlAction" | "worktreeAction">;
type WorktreeActionKind = WorktreeAction["kind"];

const DEFAULT_TIMEOUT_MS = 90_000;
// The host bounds a WSL hello at 12 s; a shorter UI tier lets the hello runner retry sooner.
const CLIENT_HELLO_TIMEOUT_MS = 30_000;
const BUILD_BUNDLE_TIMEOUT_MS = 330_000;
const SOURCE_CONTROL_READ_TIMEOUT_MS = 150_000;
const IMPORT_FILES_TIMEOUT_MS = 330_000;
const WORKTREE_ENTRY_TIMEOUT_MS = 90_000;

export const COMMAND_TIMEOUT_MS: Readonly<Record<FlatCommandType, number>> = {
  watchRepo: DEFAULT_TIMEOUT_MS,
  refresh: DEFAULT_TIMEOUT_MS,
  stageFile: DEFAULT_TIMEOUT_MS,
  readTextFile: DEFAULT_TIMEOUT_MS,
  readImageFile: DEFAULT_TIMEOUT_MS,
  buildBundle: BUILD_BUNDLE_TIMEOUT_MS,
  inspectBundleEntry: DEFAULT_TIMEOUT_MS,
  cancelBundleBuild: DEFAULT_TIMEOUT_MS,
  clientHello: CLIENT_HELLO_TIMEOUT_MS,
  setOptions: DEFAULT_TIMEOUT_MS,
  getRepoTopLevel: DEFAULT_TIMEOUT_MS,
  listRepoDirectory: DEFAULT_TIMEOUT_MS,
  listBundles: DEFAULT_TIMEOUT_MS,
  getTrFleetStatus: DEFAULT_TIMEOUT_MS,
  trFleetAction: DEFAULT_TIMEOUT_MS,
  sourceControlStatus: SOURCE_CONTROL_READ_TIMEOUT_MS,
  sourceControlDiff: SOURCE_CONTROL_READ_TIMEOUT_MS,
  sourceControlImageDiff: SOURCE_CONTROL_READ_TIMEOUT_MS,
  importFiles: IMPORT_FILES_TIMEOUT_MS,
};

// Mutations reconcile unknown outcomes; only read-only inspections use request cancellation.
export const SOURCE_CONTROL_ACTION_TIMEOUT_MS: Readonly<Record<SourceControlActionKind, number>> = {
  stage: 310_000,
  unstage: 310_000,
  discard: 370_000,
  commit: 410_000,
  push: 450_000,
  pull: 450_000,
};

// Delete/move/rename touch the worktree only; a recursive copy reuses the import budget.
export const WORKTREE_ACTION_TIMEOUT_MS: Readonly<Record<WorktreeActionKind, number>> = {
  delete: WORKTREE_ENTRY_TIMEOUT_MS,
  move: WORKTREE_ENTRY_TIMEOUT_MS,
  rename: WORKTREE_ENTRY_TIMEOUT_MS,
  copy: IMPORT_FILES_TIMEOUT_MS,
};

/** Also the reconciliation budget: how long an unknown outcome may be chased before settling. */
export function sourceControlActionTimeoutMs(kind: SourceControlActionKind): number {
  return SOURCE_CONTROL_ACTION_TIMEOUT_MS[kind];
}

/** Mutation timeouts abandon the promise; inspection timeouts also cancel the remote scan. */
export function getRequestTimeoutMs(command: UiCommand): number {
  switch (command.type) {
    case "sourceControlAction":
      return sourceControlActionTimeoutMs(command.action.kind);
    case "worktreeAction":
      return WORKTREE_ACTION_TIMEOUT_MS[command.action.kind];
    default:
      return COMMAND_TIMEOUT_MS[command.type];
  }
}
