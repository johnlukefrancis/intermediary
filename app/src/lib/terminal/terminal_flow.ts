// Path: app/src/lib/terminal/terminal_flow.ts
// Description: Per-session monotonic output credit acknowledgements: coalesced after xterm parses bytes, or on receipt while the page is hidden

import { ackTerminal } from "./terminal_ipc.js";

export interface AckCoalescer {
  /** Bytes arrived on the channel (before xterm parsed them) */
  received(bytes: number): void;
  /** xterm's write callback fired for these bytes */
  consumed(bytes: number): void;
  /** Stops acknowledgement retries and follow-up work for a disposed or ended session */
  dispose(): void;
}

/** Retry delays are bounded so a failed IPC route cannot leave a permanent timer chain. */
const ACK_RETRY_DELAYS_MS = [50, 100, 250, 500, 1_000] as const;

/**
 * Tracks monotonic desired and confirmed watermarks. Visible page: the desired watermark trails
 * xterm's parser, one flush per event-loop turn. Hidden page: Chromium throttles timers so xterm's
 * write loop stalls; the desired watermark advances on receipt (and to everything received when
 * the page hides) so a minimised window never blocks the pty.
 *
 * Only one invoke may be in flight. A failed invoke leaves the confirmed watermark untouched and
 * retries the latest desired watermark after a bounded, failure-triggered delay. This is important
 * because the backend rejects impossible watermarks and ignores stale ones: advancing before a
 * successful invoke would permanently lose the credit if the request failed.
 */
export function createAckCoalescer(sessionId: string): AckCoalescer {
  let receivedTotal = 0;
  let consumedTotal = 0;
  let desiredTotal = 0;
  let confirmedTotal = 0;
  let flushQueued = false;
  let retryTimer: ReturnType<typeof setTimeout> | null = null;
  let retryAttempt = 0;
  let inFlight = false;
  let disposed = false;
  let warned = false;

  const queueFlush = (): void => {
    if (
      disposed ||
      flushQueued ||
      inFlight ||
      retryTimer !== null ||
      desiredTotal <= confirmedTotal
    ) {
      return;
    }
    flushQueued = true;
    queueMicrotask(() => {
      flushQueued = false;
      pump();
    });
  };

  const advanceDesired = (watermark: number): void => {
    if (watermark <= desiredTotal) return;
    desiredTotal = watermark;
    queueFlush();
  };

  const scheduleRetry = (): void => {
    if (disposed || retryTimer !== null) return;
    const delay = ACK_RETRY_DELAYS_MS[Math.min(retryAttempt, ACK_RETRY_DELAYS_MS.length - 1)];
    retryAttempt += 1;
    retryTimer = setTimeout(() => {
      retryTimer = null;
      pump();
    }, delay);
  };

  const pump = (): void => {
    if (
      disposed ||
      inFlight ||
      retryTimer !== null ||
      desiredTotal <= confirmedTotal
    ) {
      return;
    }

    const watermark = desiredTotal;
    inFlight = true;
    void ackTerminal(sessionId, watermark).then(
      () => {
        inFlight = false;
        if (disposed) return;
        if (watermark > confirmedTotal) confirmedTotal = watermark;
        retryAttempt = 0;
        queueFlush();
      },
      (error: unknown) => {
        inFlight = false;
        if (disposed) return;
        if (!warned) {
          warned = true;
          console.warn(`[terminal] ack failed for ${sessionId}:`, error);
        }
        scheduleRetry();
      }
    );
  };

  const onVisibilityChange = (): void => {
    if (document.hidden && !disposed) advanceDesired(receivedTotal);
  };
  document.addEventListener("visibilitychange", onVisibilityChange);

  return {
    received(bytes) {
      if (disposed || !Number.isFinite(bytes) || bytes <= 0) return;
      receivedTotal += bytes;
      if (document.hidden) advanceDesired(receivedTotal);
    },
    consumed(bytes) {
      if (disposed || !Number.isFinite(bytes) || bytes <= 0) return;
      const nextConsumed = Math.min(receivedTotal, consumedTotal + bytes);
      if (nextConsumed <= consumedTotal) return;
      consumedTotal = nextConsumed;
      if (!document.hidden) advanceDesired(consumedTotal);
    },
    dispose() {
      disposed = true;
      if (retryTimer !== null) {
        clearTimeout(retryTimer);
        retryTimer = null;
      }
      flushQueued = false;
      document.removeEventListener("visibilitychange", onVisibilityChange);
    },
  };
}
