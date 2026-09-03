// Path: app/src/hooks/source_control/source_control_refresh.ts
// Description: Trailing-debounced status refresh scheduler with in-flight dirty flag and post-mutation de-dup

interface RefreshSchedulerOptions {
  debounceMs: number;
  /** Fetches the current repo's status; must never reject */
  run: () => Promise<void>;
}

export interface RefreshScheduler {
  /** A `sourceControlChanged` event arrived at `at`; refetch after a trailing debounce */
  notifyChanged(at: number): void;
  /** Hello / repo / epoch / focus / manual refresh: runs now or queues behind in-flight work */
  requestRefresh(): void;
  /** Status fetches queue until the action finishes (its result replaces the status) */
  actionStarted(): void;
  /**
   * `appliedAt` is when the action's fresh status replaced the snapshot, so debounced
   * refetches whose events predate it are dropped; `null` (no status applied) keeps them.
   */
  actionFinished(appliedAt: number | null): void;
  /** Repo switch or (re)mount: forget queued work; a stale in-flight run no longer blocks the next one */
  reset(): void;
  dispose(): void;
}

export function createRefreshScheduler({
  debounceMs,
  run,
}: RefreshSchedulerOptions): RefreshScheduler {
  let timer: ReturnType<typeof setTimeout> | null = null;
  let latestEventAt = 0;
  let forced = false;
  let fetchInFlight = false;
  let actionPending = false;
  let lastMutationCompletedAt = 0;
  let runSeq = 0;
  let disposed = false;

  function clearTimer(): void {
    if (timer === null) return;
    clearTimeout(timer);
    timer = null;
  }

  function hasWork(): boolean {
    return forced || latestEventAt > 0;
  }

  function drain(): void {
    if (disposed) return;
    if (!forced && latestEventAt <= lastMutationCompletedAt) {
      latestEventAt = 0;
      return;
    }
    if (fetchInFlight || actionPending) return;

    forced = false;
    latestEventAt = 0;
    fetchInFlight = true;
    runSeq += 1;
    const seq = runSeq;
    const settle = (): void => {
      if (seq !== runSeq) return;
      fetchInFlight = false;
      if (hasWork()) drain();
    };
    run().then(settle, settle);
  }

  return {
    notifyChanged(at) {
      latestEventAt = Math.max(latestEventAt, at);
      clearTimer();
      timer = setTimeout(() => {
        timer = null;
        drain();
      }, debounceMs);
    },
    requestRefresh() {
      forced = true;
      clearTimer();
      drain();
    },
    actionStarted() {
      actionPending = true;
    },
    actionFinished(appliedAt) {
      actionPending = false;
      if (appliedAt !== null) lastMutationCompletedAt = appliedAt;
      if (hasWork()) drain();
    },
    reset() {
      disposed = false;
      clearTimer();
      latestEventAt = 0;
      forced = false;
      fetchInFlight = false;
      actionPending = false;
      lastMutationCompletedAt = 0;
      runSeq += 1;
    },
    dispose() {
      disposed = true;
      clearTimer();
    },
  };
}
