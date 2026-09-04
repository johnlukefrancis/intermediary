// Path: app/src/lib/terminal/terminal_session_io.ts
// Description: One pty lifetime for a terminal tab: open handshake, queued and serialised input, output pump with credit acks, debounced resize, close

import type { RepoRoot } from "../../shared/config.js";
import {
  closeTerminal,
  createTerminalOutputChannel,
  openTerminal,
  resizeTerminal,
  writeTerminal,
  type TerminalExitFrame,
  type TerminalOpened,
  type TerminalOutputFrame,
} from "./terminal_ipc.js";
import { createAckCoalescer, type AckCoalescer } from "./terminal_flow.js";

/** Trailing debounce so a window drag does not storm ConPTY with resizes */
const RESIZE_DEBOUNCE_MS = 75;

interface Size {
  cols: number;
  rows: number;
}

export interface TerminalIoSink {
  onOpened(opened: TerminalOpened): void;
  onOpenFailed(message: string): void;
  /** Feed bytes to xterm; call `done` once parsed so the credit is returned */
  onOutput(bytes: Uint8Array, done: () => void): void;
  onExit(frame: TerminalExitFrame): void;
}

export function errorText(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return String(error);
}

export class TerminalSessionIo {
  readonly sessionId: string = crypto.randomUUID();
  private readonly sink: TerminalIoSink;
  private readonly flow: AckCoalescer;
  private readonly openPromise: Promise<void>;
  private opened = false;
  private openFailed = false;
  /** The backend ended the session (exit frame seen) */
  private ended = false;
  /** The owner let go; callbacks are ignored from here on */
  private disposed = false;
  private closing = false;
  private inFlight = false;
  private queue: Uint8Array[] = [];
  private queuedBytes = 0;
  private writeWarned = false;
  private resizeTimer: ReturnType<typeof setTimeout> | null = null;
  private pendingSize: Size | null = null;
  private sentSize: Size;

  constructor(repoRoot: RepoRoot, cols: number, rows: number, sink: TerminalIoSink) {
    this.sink = sink;
    this.sentSize = { cols, rows };
    this.flow = createAckCoalescer(this.sessionId);
    const channel = createTerminalOutputChannel(
      (frame) => {
        this.handleFrame(frame);
      },
      (message) => {
        if (!this.disposed) console.warn(`[terminal] ${message}`);
      }
    );
    this.openPromise = openTerminal({ sessionId: this.sessionId, repoRoot, cols, rows }, channel).then(
      (opened) => {
        this.handleOpened(opened);
      },
      (error: unknown) => {
        this.handleOpenFailed(errorText(error));
      }
    );
  }

  /** Queues until the open resolves, then one invoke in flight at a time (invokes are unordered) */
  writeInput(bytes: Uint8Array): void {
    if (this.disposed || this.ended || bytes.byteLength === 0) return;
    this.queue.push(bytes);
    this.queuedBytes += bytes.byteLength;
    this.pump();
  }

  /** Latest size wins after a quiet period; sent only when it differs from the pty's size */
  resize(cols: number, rows: number): void {
    if (this.disposed || this.ended) return;
    this.pendingSize = { cols, rows };
    if (!this.opened) return;
    this.clearResizeTimer();
    this.resizeTimer = setTimeout(() => {
      this.resizeTimer = null;
      this.flushResize();
    }, RESIZE_DEBOUNCE_MS);
  }

  /** Ends the pty once the open has settled (so the backend knows the id); idempotent */
  close(): void {
    if (this.closing) return;
    this.closing = true;
    void this.openPromise
      .then(() => {
        if (this.ended || this.openFailed) return;
        return closeTerminal(this.sessionId).then((outcome) => {
          if (outcome.outcome === "stillAlive") {
            console.warn(`[terminal] session ${this.sessionId} still alive after close budget`);
          }
        });
      })
      .catch((error: unknown) => {
        // The reader finished first (exit frame in flight): the backend already forgot the id
        if (errorText(error).startsWith("Unknown terminal session")) return;
        console.warn(`[terminal] close failed for ${this.sessionId}: ${errorText(error)}`);
      });
    this.dispose();
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.clearResizeTimer();
    this.flow.dispose();
    this.queue = [];
    this.queuedBytes = 0;
  }

  private handleOpened(opened: TerminalOpened): void {
    if (this.disposed || this.ended) return;
    this.opened = true;
    this.sink.onOpened(opened);
    this.pump();
    this.flushResize();
  }

  private handleOpenFailed(message: string): void {
    this.openFailed = true;
    if (this.disposed) return;
    this.flow.dispose();
    this.sink.onOpenFailed(message);
  }

  private handleFrame(frame: TerminalOutputFrame): void {
    if (frame.kind === "exit") {
      if (frame.frame.sessionId !== this.sessionId || this.ended) return;
      // Recorded even after dispose so a pending close knows the backend already finished
      this.ended = true;
      this.clearResizeTimer();
      this.flow.dispose();
      if (!this.disposed) this.sink.onExit(frame.frame);
      return;
    }
    if (this.disposed || this.ended) return;
    const size = frame.bytes.byteLength;
    this.flow.received(size);
    this.sink.onOutput(frame.bytes, () => {
      this.flow.consumed(size);
    });
  }

  private pump(): void {
    if (!this.opened || this.inFlight || this.disposed || this.ended) return;
    if (this.queue.length === 0) return;
    const merged = new Uint8Array(this.queuedBytes);
    let offset = 0;
    for (const chunk of this.queue) {
      merged.set(chunk, offset);
      offset += chunk.byteLength;
    }
    this.queue = [];
    this.queuedBytes = 0;
    this.inFlight = true;
    void writeTerminal(this.sessionId, merged)
      .catch((error: unknown) => {
        if (this.disposed || this.ended || this.writeWarned) return;
        this.writeWarned = true;
        console.warn(`[terminal] write failed for ${this.sessionId}: ${errorText(error)}`);
      })
      .finally(() => {
        this.inFlight = false;
        this.pump();
      });
  }

  private flushResize(): void {
    const size = this.pendingSize;
    this.pendingSize = null;
    if (size === null || this.disposed || this.ended || !this.opened) return;
    if (size.cols === this.sentSize.cols && size.rows === this.sentSize.rows) return;
    this.sentSize = size;
    void resizeTerminal(this.sessionId, size.cols, size.rows).catch((error: unknown) => {
      if (this.disposed || this.ended) return;
      console.warn(`[terminal] resize failed for ${this.sessionId}: ${errorText(error)}`);
    });
  }

  private clearResizeTimer(): void {
    if (this.resizeTimer === null) return;
    clearTimeout(this.resizeTimer);
    this.resizeTimer = null;
  }
}
