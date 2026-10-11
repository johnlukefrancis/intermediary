// Path: app/src/lib/agent/client_hello_runner.ts
// Description: Single-flight clientHello owner for one connection session with bounded retry backoff

import type { ClientHelloResult } from "../../shared/protocol.js";

const HELLO_RETRY_BASE_MS = 1_000;
const HELLO_RETRY_CAP_MS = 30_000;

export type HelloOutcome =
  | { kind: "ok"; result: ClientHelloResult }
  | { kind: "error"; message: string };

/** Schedules a callback and returns its cancel function. */
export type HelloScheduler = (run: () => void, delayMs: number) => () => void;

export interface ClientHelloRunnerDeps {
  send: () => Promise<ClientHelloResult>;
  onStart: () => void;
  onOutcome: (outcome: HelloOutcome) => void;
  schedule: HelloScheduler;
}

export function computeHelloRetryDelayMs(attempt: number): number {
  const delay = HELLO_RETRY_BASE_MS * Math.pow(2, Math.max(0, attempt));
  return Math.min(HELLO_RETRY_CAP_MS, delay);
}

/** Requests during an in-flight hello coalesce into one trailing hello sent after it settles. */
export class ClientHelloRunner {
  private readonly deps: ClientHelloRunnerDeps;
  private inFlight = false;
  private runQueued = false;
  private queuedWaiters: Array<(ok: boolean) => void> = [];
  private cancelRetryTimer: (() => void) | null = null;
  private retryAttempt = 0;
  private lastOnlineGeneration: number | null = null;
  private disposed = false;

  constructor(deps: ClientHelloRunnerDeps) {
    this.deps = deps;
  }

  /** Resolves with the outcome of the first hello sent after this call. */
  request(): Promise<boolean> {
    if (this.disposed) {
      return Promise.resolve(false);
    }
    return new Promise<boolean>((resolve) => {
      this.queuedWaiters.push(resolve);
      this.runQueued = true;
      this.pump();
    });
  }

  /** Re-hellos once per WSL connection generation so WSL repos' watched ids arrive. */
  noteWslOnline(generation: number): void {
    if (this.disposed || this.lastOnlineGeneration === generation) {
      return;
    }
    this.lastOnlineGeneration = generation;
    void this.request();
  }

  dispose(): void {
    this.disposed = true;
    this.cancelRetry();
    const waiters = this.queuedWaiters;
    this.queuedWaiters = [];
    this.runQueued = false;
    for (const resolve of waiters) resolve(false);
  }

  private pump(): void {
    if (this.disposed || this.inFlight || !this.runQueued) {
      return;
    }
    this.cancelRetry();
    this.runQueued = false;
    const waiters = this.queuedWaiters;
    this.queuedWaiters = [];
    this.inFlight = true;
    this.deps.onStart();
    void this.deps.send().then(
      (result) => { this.settle(waiters, { kind: "ok", result }); },
      (err: unknown) => {
        const message = err instanceof Error ? err.message : "clientHello failed";
        this.settle(waiters, { kind: "error", message });
      }
    );
  }

  private settle(waiters: Array<(ok: boolean) => void>, outcome: HelloOutcome): void {
    this.inFlight = false;
    const ok = outcome.kind === "ok";
    if (this.disposed) {
      for (const resolve of waiters) resolve(false);
      return;
    }
    this.deps.onOutcome(outcome);
    for (const resolve of waiters) resolve(ok);
    if (ok) {
      this.retryAttempt = 0;
    } else if (!this.runQueued) {
      this.scheduleRetry();
    }
    this.pump();
  }

  private scheduleRetry(): void {
    const delay = computeHelloRetryDelayMs(this.retryAttempt);
    this.retryAttempt += 1;
    this.cancelRetryTimer = this.deps.schedule(() => {
      this.cancelRetryTimer = null;
      this.runQueued = true;
      this.pump();
    }, delay);
  }

  private cancelRetry(): void {
    if (this.cancelRetryTimer === null) return;
    this.cancelRetryTimer();
    this.cancelRetryTimer = null;
  }
}
