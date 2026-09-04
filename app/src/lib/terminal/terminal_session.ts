// Path: app/src/lib/terminal/terminal_session.ts
// Description: One terminal tab living outside React: the xterm instance and wrapper element, renderer adopt/park, fit, pty lifecycle and snapshot

import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { Unicode11Addon } from "@xterm/addon-unicode11";
import type { RepoRoot } from "../../shared/config.js";
import type { TerminalCloseReason, TerminalExitFrame, TerminalOpened } from "./terminal_ipc.js";
import type { TerminalTabSnapshot, TerminalTabStatus } from "./terminal_types.js";
import { attachTerminalKeyPolicy, type TerminalKeyPolicyHandle } from "./terminal_keys.js";
import { parkElement } from "./terminal_parking.js";
import { attachWebglRenderer, type RendererHandle } from "./terminal_renderer.js";
import { hasVisibleOutput } from "./terminal_output_scan.js";
import { TerminalSessionIo } from "./terminal_session_io.js";
import {
  buildTerminalOptions,
  buildWindowsPty,
  readTerminalFontFamily,
  readTerminalTheme,
} from "./terminal_theme.js";

const encoder = new TextEncoder();

/** xterm's onBinary carries one byte per char code */
function latin1Bytes(data: string): Uint8Array {
  const bytes = new Uint8Array(data.length);
  for (let index = 0; index < data.length; index += 1) {
    bytes[index] = data.charCodeAt(index) & 0xff;
  }
  return bytes;
}

export interface TerminalSessionInit {
  repoId: string;
  ordinal: number;
  repoRoot: RepoRoot;
  foreground: boolean;
  onChange: (session: TerminalSession) => void;
}

export class TerminalSession {
  readonly tabId: string = crypto.randomUUID();
  readonly repoId: string;
  readonly ordinal: number;
  readonly label: string;
  readonly element: HTMLDivElement;
  private readonly repoRoot: RepoRoot;
  private readonly terminal: Terminal;
  private readonly fitAddon = new FitAddon();
  private readonly keys: TerminalKeyPolicyHandle;
  private readonly onChange: (session: TerminalSession) => void;
  private renderer: RendererHandle | null = null;
  private io: TerminalSessionIo | null = null;
  /** A pty open is owed; it starts on the first fit that measures a real size */
  private pendingOpen = true;
  private fitRetry: number | null = null;
  private adopted = false;
  private disposed = false;
  private status: TerminalTabStatus = "starting";
  private title: string;
  private exitCode: number | null = null;
  private exitReason: TerminalCloseReason | null = null;
  private error: string | null = null;
  private snapshot: TerminalTabSnapshot | null = null;

  constructor(init: TerminalSessionInit) {
    this.repoId = init.repoId;
    this.ordinal = init.ordinal;
    this.repoRoot = init.repoRoot;
    this.onChange = init.onChange;
    this.label = `PWSH ${init.ordinal}`;
    this.title = this.label;
    this.element = document.createElement("div");
    this.element.className = "terminal-session";
    this.element.style.cssText = "position:absolute;inset:0;";
    parkElement(this.element);

    this.terminal = new Terminal({ ...buildTerminalOptions(null), cursorBlink: init.foreground });
    this.terminal.loadAddon(this.fitAddon);
    this.terminal.loadAddon(new Unicode11Addon());
    this.terminal.unicode.activeVersion = "11";
    this.terminal.open(this.element);
    this.terminal.onData((data) => {
      this.sendInput(encoder.encode(data));
    });
    this.terminal.onBinary((data) => {
      this.sendInput(latin1Bytes(data));
    });
    this.terminal.onResize(({ cols, rows }) => {
      this.io?.resize(cols, rows);
    });
    this.terminal.onTitleChange((title) => {
      this.title = title.length > 0 ? title : this.label;
      this.changed();
    });
    this.keys = attachTerminalKeyPolicy(this.terminal, this.element, (bytes) => {
      this.sendInput(bytes);
    });
  }

  getSnapshot(): TerminalTabSnapshot {
    this.snapshot ??= Object.freeze({
      tabId: this.tabId,
      ordinal: this.ordinal,
      label: this.label,
      title: this.title,
      status: this.status,
      exitCode: this.exitCode,
      exitReason: this.exitReason,
      error: this.error,
    });
    return this.snapshot;
  }

  /**
   * Moves into the visible host, applies the deck theme, fits (opening the pty if owed) and attaches
   * WebGL on the first adopt. The renderer stays attached while parked: the addon cannot release its
   * GL context on dispose (the canvas lingers until GC), so re-creating it per switch would leak
   * contexts and rebuild the glyph atlas, while an off-screen renderer is paused and costs nothing.
   */
  adopt(host: HTMLElement): void {
    if (this.disposed) return;
    if (this.element.parentElement !== host) host.appendChild(this.element);
    this.adopted = true;
    this.applyTheme();
    this.fit();
    this.renderer ??= attachWebglRenderer(this.terminal, () => {
      this.renderer = null;
    });
  }

  /** Back to the off-screen host; the pty, scrollback and renderer stay alive */
  park(): void {
    if (this.disposed) return;
    this.adopted = false;
    parkElement(this.element);
  }

  fit(): void {
    if (this.disposed || !this.adopted) return;
    const rect = this.element.getBoundingClientRect();
    // No size yet (layout pending): never a guessed 80x24. The resize observer retries on any
    // size change; an owed open also retries next frame so a restart cannot latch unopened.
    if (rect.width < 1 || rect.height < 1 || this.fitAddon.proposeDimensions() === undefined) {
      if (this.pendingOpen) this.retryFitNextFrame();
      return;
    }
    this.fitAddon.fit();
    if (this.pendingOpen) this.startPty();
  }

  private retryFitNextFrame(): void {
    if (this.fitRetry !== null) return;
    this.fitRetry = requestAnimationFrame(() => {
      this.fitRetry = null;
      this.fit();
    });
  }

  focus(): void {
    if (!this.disposed && this.adopted) this.terminal.focus();
  }

  applyTheme(): void {
    if (this.disposed) return;
    this.terminal.options.theme = readTerminalTheme();
    const fontFamily = readTerminalFontFamily();
    if (fontFamily !== null && fontFamily !== this.terminal.options.fontFamily) {
      this.terminal.options.fontFamily = fontFamily;
    }
  }

  setCursorBlink(blink: boolean): void {
    if (!this.disposed) this.terminal.options.cursorBlink = blink;
  }

  /** Fresh pty into the same xterm (scrollback kept) for an exited or failed tab */
  restart(): void {
    if (this.disposed || (this.status !== "exited" && this.status !== "failed")) return;
    this.io?.dispose();
    this.io = null;
    this.status = "starting";
    this.exitCode = null;
    this.exitReason = null;
    this.error = null;
    this.pendingOpen = true;
    this.changed();
    this.fit();
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    if (this.fitRetry !== null) cancelAnimationFrame(this.fitRetry);
    this.io?.close();
    this.io = null;
    this.dropRenderer();
    this.keys.dispose();
    this.terminal.dispose();
    this.element.remove();
  }

  private sendInput(bytes: Uint8Array): void {
    this.io?.writeInput(bytes);
  }

  private startPty(): void {
    this.pendingOpen = false;
    const io = new TerminalSessionIo(this.repoRoot, this.terminal.cols, this.terminal.rows, {
      onOpened: (opened) => {
        this.handleOpened(io, opened);
      },
      onOpenFailed: (message) => {
        this.handleOpenFailed(io, message);
      },
      onOutput: (bytes, done) => {
        // The shell is "running" once it paints, not once it was spawned: the profile can
        // take seconds, and the STARTING notice covers that blank stretch honestly
        if (this.status === "starting" && io === this.io && hasVisibleOutput(bytes)) {
          this.status = "running";
          this.changed();
        }
        this.terminal.write(bytes, done);
      },
      onExit: (frame) => {
        this.handleExit(io, frame);
      },
    });
    this.io = io;
  }

  private handleOpened(io: TerminalSessionIo, opened: TerminalOpened): void {
    if (io !== this.io) return;
    this.terminal.options.windowsPty = buildWindowsPty(opened.windowsBuildNumber);
  }

  private handleOpenFailed(io: TerminalSessionIo, message: string): void {
    if (io !== this.io) return;
    this.status = "failed";
    this.error = message;
    this.changed();
  }

  private handleExit(io: TerminalSessionIo, frame: TerminalExitFrame): void {
    if (io !== this.io) return;
    this.status = "exited";
    this.exitCode = frame.code;
    this.exitReason = frame.reason;
    this.changed();
  }

  private dropRenderer(): void {
    const renderer = this.renderer;
    this.renderer = null;
    renderer?.dispose();
  }

  private changed(): void {
    this.snapshot = null;
    this.onChange(this);
  }
}
