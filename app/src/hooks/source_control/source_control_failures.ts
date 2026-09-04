// Path: app/src/hooks/source_control/source_control_failures.ts
// Description: Route an agent rejection by the effect certainty it carries, never by its error code namespace

import {
  AgentResponseError,
  agentErrorDetails,
  parseAgentErrorCode,
} from "../../lib/agent/error_codes.js";
import {
  SOURCE_CONTROL_STATE_CHANGED_CODE,
  SourceControlErrorDetailsSchema,
  type SourceControlActionKind,
  type SourceControlEffect,
} from "../../shared/protocol.js";
import type { SourceControlActionError } from "./source_control_types.js";

/** What the hook does next after one action failed */
export type ActionOutcome =
  /** The agent does not know the command at all: nothing ran, and the view is unusable */
  | { kind: "agentUpdateRequired"; message: string }
  /** The agent proved the action had no effect; Git's own text is shown inline */
  | { kind: "rejected"; error: SourceControlActionError; refreshNow: boolean }
  /** The action may have landed: chase the repository until it is quiet again, showing Git's own text when the agent answered */
  | { kind: "reconcile"; error: SourceControlActionError | null };

/** Text after "<CODE>: " when the agent answered, otherwise the raw transport message */
export function agentErrorMessage(error: unknown): string {
  if (error instanceof AgentResponseError) return error.serverMessage;
  return error instanceof Error ? error.message : String(error);
}

/** Effect certainty the agent attached to this error; null when it said nothing usable */
function actionEffect(error: unknown): SourceControlEffect | null {
  const parsed = SourceControlErrorDetailsSchema.safeParse(agentErrorDetails(error));
  return parsed.success ? parsed.data.effect : null;
}

/**
 * Only `details.effect: "notApplied"` is proof that nothing happened. The `GIT_` namespace says
 * which Git command complained, never whether it crossed its effect boundary first: a commit
 * whose post-commit hook overran the command budget reports `GIT_TIMEOUT` and still landed.
 */
export function actionOutcome(action: SourceControlActionKind, error: unknown): ActionOutcome {
  const code = parseAgentErrorCode(error);
  if (code === "UNKNOWN_COMMAND") {
    return { kind: "agentUpdateRequired", message: agentErrorMessage(error) };
  }
  // A transport failure carries no code, and an older agent carries no effect: both are unknown.
  if (code === null) return { kind: "reconcile", error: null };
  if (actionEffect(error) !== "notApplied") {
    // A remote rejection ("! [remote rejected] … Working directory has unstaged changes") is the
    // common case here: Git said exactly why, and that text must survive the reconciliation.
    return {
      kind: "reconcile",
      error: { action, code, message: agentErrorMessage(error), uncertain: true },
    };
  }
  return {
    kind: "rejected",
    error: { action, code, message: agentErrorMessage(error) },
    // The snapshot this action was reviewed against is stale by definition: read the repo again.
    refreshNow: code === SOURCE_CONTROL_STATE_CHANGED_CODE,
  };
}
