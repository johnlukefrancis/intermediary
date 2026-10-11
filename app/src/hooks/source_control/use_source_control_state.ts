// Path: app/src/hooks/source_control/use_source_control_state.ts
// Description: SOURCE write side over the shared status registry: serialized actions, reconciliation, notices, draft

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type {
  SourceControlAction,
  SourceControlActionKind,
} from "../../shared/protocol.js";
import { sendSourceControlAction } from "../../lib/agent/messages_source_control.js";
import { statusRegistry } from "../../lib/source_control/status_registry.js";
import { useAgent } from "../use_agent.js";
import { actionOutcome } from "./source_control_failures.js";
import { countChangedPaths } from "./source_control_counts.js";
import { useReconciliation } from "./source_control_reconcile.js";
import { useSourceControlCommands } from "./source_control_commands.js";
import { useSourceControlStatus } from "./use_source_control_status.js";
import { totalConflictCount } from "../../lib/source_control/conflict_count.js";
import type {
  SourceControlActionError,
  SourceControlCommit,
  SourceControlPhase,
  SourceControlState,
} from "./source_control_types.js";

export function useSourceControlState(repoId: string): SourceControlState {
  const { client, connectionState, helloState } = useAgent();
  const view = useSourceControlStatus(repoId);
  const [reconciling, setReconciling] = useState<SourceControlActionKind | null>(null);
  const [pendingAction, setPendingAction] = useState<SourceControlActionKind | null>(null);
  const [actionError, setActionError] = useState<SourceControlActionError | null>(null);
  const [hookNotice, setHookNotice] = useState<string[] | null>(null);
  const [hookAddedNotice, setHookAddedNotice] = useState<string[] | null>(null);
  const [lastCommit, setLastCommit] = useState<SourceControlCommit | null>(null);
  const [commitMessage, setCommitMessage] = useState("");

  const repoIdRef = useRef(repoId);
  repoIdRef.current = repoId;
  const pendingActionRef = useRef<SourceControlActionKind | null>(null);

  const isReady =
    client !== null && connectionState.status === "connected" && helloState.status === "ok";

  const reportCommit = useCallback((commit: SourceControlCommit): void => {
    setLastCommit(commit);
    setCommitMessage("");
  }, []);
  const reconciliation = useReconciliation(statusRegistry.schedulerFor(repoId), reportCommit);

  // Every read reaches reconciliation, including one taken while a mutation still runs.
  useEffect(
    () =>
      statusRegistry.attachWriter(repoId, {
        acceptStatus(status) {
          const next = reconciliation.acceptStatus(status);
          setReconciling(next.kind === "reconciling" ? next.action : null);
        },
        acceptFailure() {
          const kept = reconciliation.acceptFailure();
          if (!kept) setReconciling(null);
          return kept;
        },
      }),
    [reconciliation, repoId]
  );

  const runAction = useCallback(
    async (action: SourceControlAction): Promise<void> => {
      if (client === null || !isReady || pendingActionRef.current !== null) return;
      const requestRepoId = repoIdRef.current;
      const isCurrent = (): boolean => repoIdRef.current === requestRepoId;
      const previousHeadSha = statusRegistry.getSlot(requestRepoId).status?.headSha ?? null;
      pendingActionRef.current = action.kind;
      setPendingAction(action.kind);
      setActionError(null);
      setHookNotice(null);
      setHookAddedNotice(null);
      statusRegistry.actionStarted(requestRepoId);
      let appliedAt: number | null = null;
      let refreshAfterReject = false;
      try {
        const result = await sendSourceControlAction(client, requestRepoId, action);
        statusRegistry.applyStatus(requestRepoId, result.status);
        appliedAt = Date.now();
        if (!isCurrent()) return;
        if (result.kind === "commit" && result.commitSha !== undefined) {
          reportCommit({ sha: result.commitSha, at: Date.now() });
        }
        if (result.hookChangedPaths !== undefined && result.hookChangedPaths.length > 0) {
          setHookNotice(result.hookChangedPaths);
        }
        // Paths no reviewed row covered: the commit landed with content the user never saw.
        if (result.hookAddedPaths !== undefined && result.hookAddedPaths.length > 0) {
          setHookAddedNotice(result.hookAddedPaths);
        }
      } catch (err: unknown) {
        const outcome = actionOutcome(action.kind, err);
        if (outcome.kind === "agentUpdateRequired") {
          statusRegistry.applyError(requestRepoId, { kind: "error", code: "UNKNOWN_COMMAND", message: outcome.message });
        } else if (!isCurrent()) {
          return;
        } else if (outcome.kind === "rejected") {
          setActionError(outcome.error);
          refreshAfterReject = outcome.refreshNow;
        } else {
          if (outcome.error !== null) setActionError(outcome.error);
          reconciliation.begin(action.kind, previousHeadSha);
          setReconciling(action.kind);
        }
      } finally {
        // A rejected action applied no status, so queued change events must still refetch.
        statusRegistry.actionFinished(requestRepoId, appliedAt);
        if (isCurrent()) {
          pendingActionRef.current = null;
          setPendingAction(null);
          if (reconciliation.isActive() || refreshAfterReject) statusRegistry.schedulerFor(requestRepoId).requestRefresh();
        }
      }
    },
    [client, isReady, reconciliation, reportCommit]
  );

  useEffect(() => {
    pendingActionRef.current = null;
    reconciliation.clear();
    setReconciling(null);
    setPendingAction(null);
    setActionError(null);
    setHookNotice(null);
    setHookAddedNotice(null);
    setLastCommit(null);
    setCommitMessage("");
  }, [reconciliation, repoId]);

  const commands = useSourceControlCommands(runAction);
  const dismissActionError = useCallback(() => { setActionError(null); }, []);
  const dismissHookNotice = useCallback(() => { setHookNotice(null); }, []);
  const dismissHookAddedNotice = useCallback(() => { setHookAddedNotice(null); }, []);
  const refresh = useCallback(() => {
    statusRegistry.refresh(repoId);
  }, [repoId]);

  const phase = useMemo<SourceControlPhase>(
    () => (reconciling === null ? view.phase : { kind: "reconciling", action: reconciling }),
    [reconciling, view.phase]
  );
  const { status } = view;
  const changeCount = status === null ? 0 : countChangedPaths(status);
  const conflictCount = status === null ? 0 : totalConflictCount(status);

  return {
    phase,
    status,
    changeCount,
    conflictCount,
    pendingAction,
    actionError,
    hookNotice,
    hookAddedNotice,
    lastCommit,
    commitMessage,
    setCommitMessage,
    dismissActionError,
    dismissHookNotice,
    dismissHookAddedNotice,
    refresh,
    ...commands,
  };
}
