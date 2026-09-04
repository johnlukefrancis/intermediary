// Path: app/src/lib/terminal/terminal_ipc.ts
// Description: Typed Tauri invoke wrappers and the output-channel seam for terminal sessions (mirrors src-tauri terminal/frames.rs)

import { Channel, invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import type { RepoRoot } from "../../shared/config.js";

/** Header carrying the session id on the raw-body `terminal_write` invoke */
const SESSION_HEADER = "tauri-terminal-session";

export interface TerminalOpenRequest {
  /** Minted by the caller (`crypto.randomUUID()`) so writes are addressable before the open resolves */
  sessionId: string;
  repoRoot: RepoRoot;
  cols: number;
  rows: number;
}

export interface TerminalOpened {
  sessionId: string;
  pid: number;
  /** Host `CurrentBuildNumber` for xterm's ConPTY reflow; `null` off Windows */
  windowsBuildNumber: number | null;
  startDir: string;
  initialCommand: string | null;
}

/** Cumulative output watermark accepted by the backend flow gate. */
export interface TerminalAckRequest {
  sessionId: string;
  consumedTotal: number;
}

export const TerminalCloseReasonSchema = z.enum([
  "childExit",
  "closed",
  "webviewNavigation",
  "appExit",
  "readerError",
  "openFailed",
]);

export type TerminalCloseReason = z.infer<typeof TerminalCloseReasonSchema>;

/** JSON frame the backend sends on the output channel after the last output byte */
export const TerminalExitFrameSchema = z.object({
  kind: z.literal("exit"),
  sessionId: z.string(),
  code: z.number().int().nullable(),
  reason: TerminalCloseReasonSchema,
});

export type TerminalExitFrame = z.infer<typeof TerminalExitFrameSchema>;

export const TerminalCloseOutcomeSchema = z.discriminatedUnion("outcome", [
  z.object({ outcome: z.literal("exited"), code: z.number().int().nullable() }),
  z.object({ outcome: z.literal("escalated"), code: z.number().int().nullable() }),
  z.object({ outcome: z.literal("stillAlive") }),
]);

export type TerminalCloseOutcome = z.infer<typeof TerminalCloseOutcomeSchema>;

/** What arrives on the output channel: raw pty bytes, or the exit frame */
export type TerminalOutputFrame =
  | { kind: "bytes"; bytes: Uint8Array }
  | { kind: "exit"; frame: TerminalExitFrame };

/**
 * Builds the output channel. The channel payload is the one dynamic seam of the terminal:
 * raw chunks arrive as `ArrayBuffer`, the exit frame as a JSON object, both narrowed here.
 */
export function createTerminalOutputChannel(
  onFrame: (frame: TerminalOutputFrame) => void,
  onMalformed: (message: string) => void
): Channel {
  // TODO(ts-precision): Tauri channel payloads are untyped at the IPC boundary; narrowed below
  const channel = new Channel();
  channel.onmessage = (message: unknown) => {
    if (message instanceof ArrayBuffer) {
      onFrame({ kind: "bytes", bytes: new Uint8Array(message) });
      return;
    }
    const parsed = TerminalExitFrameSchema.safeParse(message);
    if (parsed.success) {
      onFrame({ kind: "exit", frame: parsed.data });
      return;
    }
    onMalformed(`Unrecognised terminal frame: ${parsed.error.message}`);
  };
  return channel;
}

export async function openTerminal(
  request: TerminalOpenRequest,
  onOutput: Channel
): Promise<TerminalOpened> {
  return invoke<TerminalOpened>("terminal_open", { request, onOutput });
}

/** Raw-body write; callers serialise writes per session because separate invokes are unordered */
export async function writeTerminal(sessionId: string, bytes: Uint8Array): Promise<void> {
  await invoke("terminal_write", bytes, { headers: { [SESSION_HEADER]: sessionId } });
}

export async function resizeTerminal(sessionId: string, cols: number, rows: number): Promise<void> {
  await invoke("terminal_resize", { sessionId, cols, rows });
}

export async function ackTerminal(sessionId: string, consumedTotal: number): Promise<void> {
  const request: TerminalAckRequest = { sessionId, consumedTotal };
  await invoke("terminal_ack", {
    sessionId: request.sessionId,
    consumedTotal: request.consumedTotal,
  });
}

export async function closeTerminal(sessionId: string): Promise<TerminalCloseOutcome> {
  const raw = await invoke<unknown>("terminal_close", { sessionId });
  return TerminalCloseOutcomeSchema.parse(raw);
}

/** Clipboard text read in Rust: WebView2 cannot read the clipboard without a permission prompt */
export async function readClipboardText(): Promise<string> {
  return invoke<string>("terminal_clipboard_text");
}
