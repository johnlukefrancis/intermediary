// Path: app/src/hooks/source_control/source_control_reconcile.ts
// Description: Bounded backoff loop that resolves an action whose outcome the UI could not observe

import { useCallback, useMemo, useRef } from "react";
import { sourceControlActionTimeoutMs } from "../../lib/agent/agent_request_timeouts.js";
import type { SourceControlActionKind, SourceControlStatus } from "../../shared/protocol.js";
import type { RefreshScheduler } from "./source_control_refresh.js";
import type { SourceControlCommit, SourceControlPhase } from "./source_control_types.js";

const BACKOFF_BASE_MS = 1_000;
const BACKOFF_CAP_MS = 8_000;

/** One unresolved action: what ran, what HEAD was before it, and how long it may be chased */
interface Reconcile {
  action: SourceControlActionKind;
  previousHeadSha: string | null;
  /** The action's own UI budget: past it the loop settles on whatever the repo now says */
  deadlineAt: number;
  attempt: number;
}

/** 1 s, 2 s, 4 s, then 8 s for every further attempt */
function backoffMs(attempt: number): number {
  return Math.min(BACKOFF_CAP_MS, BACKOFF_BASE_MS * 2 ** Math.max(0, attempt));
}

/** HEAD moved and nothing is staged any more: the commit with the unknown outcome landed */
function reconciledCommitSha(reconcile: Reconcile, status: SourceControlStatus): string | null {
  if (reconcile.action !== "commit" || status.headSha === null) return null;
  if (status.headSha === reconcile.previousHeadSha || status.committable) return null;
  return status.headSha;
}

export interface Reconciliation {
  /** An action's outcome is unknown: chase the repo until it is quiet or the budget is spent */
  begin(action: SourceControlActionKind, previousHeadSha: string | null): void;
  isActive(): boolean;
  /** The phase this status leaves the view in; `ready` when no loop is running any more */
  acceptStatus(status: SourceControlStatus): SourceControlPhase;
  /** True when a failed read scheduled another attempt; false when the loop is over or absent */
  acceptFailure(): boolean;
  clear(): void;
}

export function useReconciliation(
  scheduler: RefreshScheduler,
  onCommitReconciled: (commit: SourceControlCommit) => void
): Reconciliation {
  const stateRef = useRef<Reconcile | null>(null);

  const scheduleNext = useCallback(
    (reconcile: Reconcile): void => {
      stateRef.current = { ...reconcile, attempt: reconcile.attempt + 1 };
      scheduler.requestRefreshIn(backoffMs(reconcile.attempt));
    },
    [scheduler]
  );

  return useMemo(
    () => ({
      begin(action, previousHeadSha) {
        stateRef.current = {
          action,
          previousHeadSha,
          deadlineAt: Date.now() + sourceControlActionTimeoutMs(action),
          attempt: 0,
        };
      },
      isActive: () => stateRef.current !== null,
      acceptStatus(status) {
        const reconcile = stateRef.current;
        if (reconcile === null) return { kind: "ready" };
        const now = Date.now();
        // A status read while the mutation still holds the lock may have observed the repo
        // between two of its Git commands, so only a quiet repo ends the loop.
        if (status.mutationInProgress && now < reconcile.deadlineAt) {
          scheduleNext(reconcile);
          return { kind: "reconciling", action: reconcile.action };
        }
        stateRef.current = null;
        const sha = reconciledCommitSha(reconcile, status);
        if (sha !== null) onCommitReconciled({ sha, at: now });
        return { kind: "ready" };
      },
      acceptFailure() {
        const reconcile = stateRef.current;
        if (reconcile === null) return false;
        // A read that fails mid-reconciliation proves nothing; keep asking until the budget ends.
        if (Date.now() < reconcile.deadlineAt) {
          scheduleNext(reconcile);
          return true;
        }
        stateRef.current = null;
        return false;
      },
      clear() {
        stateRef.current = null;
      },
    }),
    [onCommitReconciled, scheduleNext]
  );
}
