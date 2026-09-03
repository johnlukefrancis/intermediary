// Path: app/src/hooks/source_control/use_source_control_state.ts
// Description: Per-repo source-control status state machine with event-driven refresh and serialized actions

import { useCallback, useEffect, useRef, useState } from "react";
import type {
  SourceControlAction,
  SourceControlActionKind,
  SourceControlStatus,
} from "../../shared/protocol.js";
import {
  sendSourceControlAction,
  sendSourceControlStatus,
} from "../../lib/agent/messages_source_control.js";
import { parseAgentErrorCode } from "../../lib/agent/error_codes.js";
import {
  computeTransientRetryDelayMs,
  isTransientWslTransportError,
} from "../../lib/agent/transient_wsl_error.js";
import { useAgent } from "../use_agent.js";
import { createRefreshScheduler, type RefreshScheduler } from "./source_control_refresh.js";
import { actionOutcome, agentErrorMessage } from "./source_control_failures.js";
import { countChangedPaths } from "./source_control_counts.js";
import { useReconciliation } from "./source_control_reconcile.js";
import { useSourceControlCommands } from "./source_control_commands.js";
import { totalConflictCount } from "../../lib/source_control/conflict_count.js";
import type {
  SourceControlActionError,
  SourceControlCommit,
  SourceControlPhase,
  SourceControlState,
} from "./source_control_types.js";

const REFRESH_DEBOUNCE_MS = 300;

interface Snapshot {
  repoId: string;
  status: SourceControlStatus;
}

export function useSourceControlState(repoId: string): SourceControlState {
  const { client, connectionState, helloState, rehydrateEpoch, subscribe } = useAgent();
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [phase, setPhase] = useState<SourceControlPhase>({ kind: "waiting" });
  const [pendingAction, setPendingAction] = useState<SourceControlActionKind | null>(null);
  const [actionError, setActionError] = useState<SourceControlActionError | null>(null);
  const [hookNotice, setHookNotice] = useState<string[] | null>(null);
  const [hookAddedNotice, setHookAddedNotice] = useState<string[] | null>(null);
  const [lastCommit, setLastCommit] = useState<SourceControlCommit | null>(null);
  const [commitMessage, setCommitMessage] = useState("");

  const repoIdRef = useRef(repoId);
  const generationRef = useRef(0);
  const pendingActionRef = useRef<SourceControlActionKind | null>(null);
  const headShaRef = useRef<string | null>(null);
  const retryAttemptRef = useRef(0);
  const fetchRef = useRef<() => Promise<void>>(() => Promise.resolve());
  const schedulerRef = useRef<RefreshScheduler | null>(null);
  schedulerRef.current ??= createRefreshScheduler({
    debounceMs: REFRESH_DEBOUNCE_MS,
    run: () => fetchRef.current(),
  });
  const scheduler = schedulerRef.current;

  if (repoIdRef.current !== repoId) {
    repoIdRef.current = repoId;
    generationRef.current += 1;
  }

  const isReady =
    client !== null && connectionState.status === "connected" && helloState.status === "ok";

  const reportCommit = useCallback((commit: SourceControlCommit): void => {
    setLastCommit(commit);
    setCommitMessage("");
  }, []);
  const reconciliation = useReconciliation(scheduler, reportCommit);

  const applyStatus = useCallback(
    (forRepoId: string, status: SourceControlStatus): void => {
      headShaRef.current = status.headSha;
      // Every fresh status is worth showing, including one read while a mutation still runs;
      // that one just cannot end a reconciliation, which is the phase's own decision.
      setSnapshot({ repoId: forRepoId, status });
      setPhase(reconciliation.acceptStatus(status));
    },
    [reconciliation]
  );

  const fetchStatus = useCallback(async (): Promise<void> => {
    if (client === null || !isReady) return;
    const requestRepoId = repoIdRef.current;
    generationRef.current += 1;
    const generation = generationRef.current;
    const isStale = (): boolean =>
      repoIdRef.current !== requestRepoId || generationRef.current !== generation;

    setPhase((current) => (current.kind === "reconciling" ? current : { kind: "loading" }));
    try {
      const result = await sendSourceControlStatus(client, requestRepoId);
      if (isStale()) return;
      retryAttemptRef.current = 0;
      applyStatus(requestRepoId, result.status);
    } catch (err: unknown) {
      if (isStale()) return;
      if (reconciliation.acceptFailure()) return;
      if (isTransientWslTransportError(err)) {
        scheduler.requestRefreshIn(computeTransientRetryDelayMs(retryAttemptRef.current));
        retryAttemptRef.current += 1;
        return;
      }
      retryAttemptRef.current = 0;
      setSnapshot(null);
      setPhase({ kind: "error", code: parseAgentErrorCode(err), message: agentErrorMessage(err) });
    }
  }, [applyStatus, client, isReady, reconciliation, scheduler]);

  useEffect(() => {
    fetchRef.current = fetchStatus;
  }, [fetchStatus]);

  const runAction = useCallback(
    async (action: SourceControlAction): Promise<void> => {
      if (client === null || !isReady || pendingActionRef.current !== null) return;
      const requestRepoId = repoIdRef.current;
      const previousHeadSha = headShaRef.current;
      pendingActionRef.current = action.kind;
      setPendingAction(action.kind);
      setActionError(null);
      setHookNotice(null);
      setHookAddedNotice(null);
      scheduler.actionStarted();
      let appliedAt: number | null = null;
      let refreshAfterReject = false;
      try {
        const result = await sendSourceControlAction(client, requestRepoId, action);
        if (repoIdRef.current !== requestRepoId) return;
        generationRef.current += 1;
        applyStatus(requestRepoId, result.status);
        appliedAt = Date.now();
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
        if (repoIdRef.current !== requestRepoId) return;
        const outcome = actionOutcome(action.kind, err);
        if (outcome.kind === "agentUpdateRequired") {
          setSnapshot(null);
          setPhase({ kind: "error", code: "UNKNOWN_COMMAND", message: outcome.message });
        } else if (outcome.kind === "rejected") {
          setActionError(outcome.error);
          refreshAfterReject = outcome.refreshNow;
        } else {
          reconciliation.begin(action.kind, previousHeadSha);
          setPhase({ kind: "reconciling", action: action.kind });
        }
      } finally {
        if (repoIdRef.current === requestRepoId) {
          pendingActionRef.current = null;
          setPendingAction(null);
          // A rejected action applied no status, so queued change events must still refetch.
          scheduler.actionFinished(appliedAt);
          if (reconciliation.isActive() || refreshAfterReject) scheduler.requestRefresh();
        }
      }
    },
    [applyStatus, client, isReady, reconciliation, reportCommit, scheduler]
  );

  useEffect(() => {
    pendingActionRef.current = null;
    headShaRef.current = null;
    retryAttemptRef.current = 0;
    reconciliation.clear();
    scheduler.reset();
    setSnapshot(null);
    setPhase({ kind: "waiting" });
    setPendingAction(null);
    setActionError(null);
    setHookNotice(null);
    setHookAddedNotice(null);
    setLastCommit(null);
    setCommitMessage("");
  }, [reconciliation, repoId, scheduler]);

  useEffect(
    () =>
      subscribe((event) => {
        if (event.type !== "sourceControlChanged" || event.repoId !== repoIdRef.current) return;
        scheduler.notifyChanged(Date.now());
      }),
    [scheduler, subscribe]
  );

  useEffect(() => {
    if (!isReady) {
      setPhase((current) => (current.kind === "reconciling" ? current : { kind: "waiting" }));
      return;
    }
    generationRef.current += 1;
    scheduler.requestRefresh();
  }, [helloState.lastHelloAt, isReady, rehydrateEpoch, repoId, scheduler]);

  useEffect(() => {
    const handleFocus = (): void => {
      if (pendingActionRef.current !== null) return;
      scheduler.requestRefresh();
    };
    window.addEventListener("focus", handleFocus);
    return () => {
      window.removeEventListener("focus", handleFocus);
    };
  }, [scheduler]);

  useEffect(
    () => () => {
      scheduler.dispose();
    },
    [scheduler]
  );

  const commands = useSourceControlCommands(runAction);
  const dismissActionError = useCallback(() => { setActionError(null); }, []);
  const dismissHookNotice = useCallback(() => { setHookNotice(null); }, []);
  const dismissHookAddedNotice = useCallback(() => { setHookAddedNotice(null); }, []);
  const refresh = useCallback(() => {
    retryAttemptRef.current = 0;
    scheduler.requestRefresh();
  }, [scheduler]);

  const status = snapshot !== null && snapshot.repoId === repoId ? snapshot.status : null;
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
