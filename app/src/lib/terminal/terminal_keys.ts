// Path: app/src/lib/terminal/terminal_keys.ts
// Description: Windows Terminal key and mouse policy for one xterm: copy/paste chords, Ctrl+C with a selection, right-click, Shift+Enter

import type { Terminal } from "@xterm/xterm";
import { readClipboardText } from "./terminal_ipc.js";

/** ESC CR: the newline chord Claude Code and other TUIs read from Shift+Enter */
const SHIFT_ENTER_BYTES = new Uint8Array([0x1b, 0x0d]);

export interface TerminalKeyPolicyHandle {
  dispose(): void;
}

type Chord = "copy" | "paste" | "interruptOrCopy" | "shiftEnter";

function chordOf(event: KeyboardEvent): Chord | null {
  if (event.altKey || event.metaKey) return null;
  const key = event.key.toLowerCase();
  if (event.ctrlKey && event.shiftKey) {
    if (key === "c") return "copy";
    if (key === "v") return "paste";
    return null;
  }
  if (event.ctrlKey) {
    if (key === "c") return "interruptOrCopy";
    if (key === "v") return "paste";
    return null;
  }
  if (event.shiftKey && event.key === "Enter") return "shiftEnter";
  return null;
}

function copySelection(terminal: Terminal): void {
  if (!terminal.hasSelection()) return;
  const text = terminal.getSelection();
  terminal.clearSelection();
  void navigator.clipboard.writeText(text).catch((error: unknown) => {
    console.warn("[terminal] clipboard write failed:", error);
  });
}

/** Reads through Rust: WebView2 blocks navigator.clipboard.readText without a permission prompt */
function pasteClipboard(terminal: Terminal): void {
  void readClipboardText().then(
    (text) => {
      if (text.length > 0) terminal.paste(text);
    },
    (error: unknown) => {
      console.warn("[terminal] clipboard read failed:", error);
    }
  );
}

/**
 * Installs the policy on the terminal and its wrapper. Returning `false` from xterm's custom
 * key handler only skips xterm's own handling, so consumed chords call `preventDefault`
 * themselves; every other key goes to xterm untouched.
 */
export function attachTerminalKeyPolicy(
  terminal: Terminal,
  surface: HTMLElement,
  sendBytes: (bytes: Uint8Array) => void
): TerminalKeyPolicyHandle {
  terminal.attachCustomKeyEventHandler((event) => {
    const chord = chordOf(event);
    if (chord === null) return true;
    if (chord === "interruptOrCopy" && !terminal.hasSelection()) return true;
    // The keypress/keyup halves of a consumed chord are swallowed too
    if (event.type !== "keydown") return false;
    event.preventDefault();
    switch (chord) {
      case "copy":
      case "interruptOrCopy":
        copySelection(terminal);
        break;
      case "paste":
        pasteClipboard(terminal);
        break;
      case "shiftEnter":
        sendBytes(SHIFT_ENTER_BYTES);
        break;
    }
    return false;
  });

  const onContextMenu = (event: MouseEvent): void => {
    event.preventDefault();
    if (terminal.hasSelection()) {
      copySelection(terminal);
    } else {
      pasteClipboard(terminal);
    }
  };
  surface.addEventListener("contextmenu", onContextMenu);

  return {
    dispose() {
      surface.removeEventListener("contextmenu", onContextMenu);
    },
  };
}
