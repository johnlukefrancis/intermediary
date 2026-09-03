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
import { agentErrorMessage, classifyActionFailure } from "./source_control_failures.js";
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

/** Set when an action's outcome is unknown; resolved by the next successful status read */
interface Reconcile {
  action: SourceControlActionKind;
  previousHeadSha: string | null;
}

export function useSourceControlState(repoId: string): SourceControlState {
  const { client, connectionState, helloState, rehydrateEpoch, subscribe } = useAgent();
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [phase, setPhase] = useState<SourceControlPhase>({ kind: "waiting" });
  const [pendingAction, setPendingAction] = useState<SourceControlActionKind | null>(null);
  const [actionError, setActionError] = useState<SourceControlActionError | null>(null);
  const [lastCommit, setLastCommit] = useState<SourceControlCommit | null>(null);
  const [commitMessage, setCommitMessage] = useState("");

  const repoIdRef = useRef(repoId);
  const generationRef = useRef(0);
  const pendingActionRef = useRef<SourceControlActionKind | null>(null);
  const reconcileRef = useRef<Reconcile | null>(null);
  const headShaRef = useRef<string | null>(null);
  const retryAttemptRef = useRef(0);
  const retryTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
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

  const clearRetryTimer = useCallback((): void => {
    if (retryTimerRef.current === null) return;
    clearTimeout(retryTimerRef.current);
    retryTimerRef.current = null;
  }, []);

  const applyStatus = useCallback((forRepoId: string, status: SourceControlStatus): void => {
    headShaRef.current = status.headSha;
    setSnapshot({ repoId: forRepoId, status });
    setPhase({ kind: "ready" });
    const reconcile = reconcileRef.current;
    if (reconcile === null) return;
    reconcileRef.current = null;
    // HEAD moved and nothing is committable any more after an unknown-outcome commit: it landed.
    if (reconcile.action !== "commit" || status.headSha === null) return;
    if (status.headSha !== reconcile.previousHeadSha && !status.committable) {
      setLastCommit({ sha: status.headSha, at: Date.now() });
      setCommitMessage("");
    }
  }, []);

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
      if (isTransientWslTransportError(err)) {
        const delay = computeTransientRetryDelayMs(retryAttemptRef.current);
        retryAttemptRef.current += 1;
        clearRetryTimer();
        retryTimerRef.current = setTimeout(() => {
          retryTimerRef.current = null;
          if (isStale()) return;
          scheduler.requestRefresh();
        }, delay);
        return;
      }
      retryAttemptRef.current = 0;
      setSnapshot(null);
      setPhase({ kind: "error", code: parseAgentErrorCode(err), message: agentErrorMessage(err) });
    }
  }, [applyStatus, clearRetryTimer, client, isReady, scheduler]);

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
      scheduler.actionStarted();
      let appliedAt: number | null = null;
      try {
        const result = await sendSourceControlAction(client, requestRepoId, action);
        if (repoIdRef.current !== requestRepoId) return;
        generationRef.current += 1;
        applyStatus(requestRepoId, result.status);
        appliedAt = Date.now();
        if (result.kind === "commit" && result.commitSha !== undefined) {
          setLastCommit({ sha: result.commitSha, at: Date.now() });
          setCommitMessage("");
        }
      } catch (err: unknown) {
        if (repoIdRef.current !== requestRepoId) return;
        const failure = classifyActionFailure(err);
        if (failure.type === "agentUpdateRequired") {
          setSnapshot(null);
          setPhase({ kind: "error", code: "UNKNOWN_COMMAND", message: failure.message });
        } else if (failure.type === "rejected") {
          setActionError({ action: action.kind, code: failure.code, message: failure.message });
        } else {
          reconcileRef.current = { action: action.kind, previousHeadSha };
          setPhase({ kind: "reconciling", action: action.kind });
        }
      } finally {
        if (repoIdRef.current === requestRepoId) {
          pendingActionRef.current = null;
          setPendingAction(null);
          // A rejected action applied no status, so queued change events must still refetch.
          scheduler.actionFinished(appliedAt);
          if (reconcileRef.current !== null) scheduler.requestRefresh();
        }
      }
    },
    [applyStatus, client, isReady, scheduler]
  );

  useEffect(() => {
    pendingActionRef.current = null;
    reconcileRef.current = null;
    headShaRef.current = null;
    retryAttemptRef.current = 0;
    clearRetryTimer();
    scheduler.reset();
    setSnapshot(null);
    setPhase({ kind: "waiting" });
    setPendingAction(null);
    setActionError(null);
    setLastCommit(null);
    setCommitMessage("");
  }, [clearRetryTimer, repoId, scheduler]);

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
      clearRetryTimer();
      setPhase((current) => (current.kind === "reconciling" ? current : { kind: "waiting" }));
      return;
    }
    generationRef.current += 1;
    scheduler.requestRefresh();
  }, [clearRetryTimer, helloState.lastHelloAt, isReady, rehydrateEpoch, repoId, scheduler]);

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
      clearRetryTimer();
    },
    [clearRetryTimer, scheduler]
  );

  const commands = useSourceControlCommands(runAction);
  const dismissActionError = useCallback(() => { setActionError(null); }, []);
  const refresh = useCallback(() => {
    retryAttemptRef.current = 0;
    scheduler.requestRefresh();
  }, [scheduler]);

  const status = snapshot !== null && snapshot.repoId === repoId ? snapshot.status : null;
  const changeCount =
    status === null ? 0 : status.index.length + status.worktree.length + status.conflicts.length;
  const conflictCount = status === null ? 0 : totalConflictCount(status);

  return {
    phase,
    status,
    changeCount,
    conflictCount,
    pendingAction,
    actionError,
    lastCommit,
    commitMessage,
    setCommitMessage,
    dismissActionError,
    refresh,
    ...commands,
  };
}
