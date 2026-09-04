// Path: app/src/lib/terminal/terminal_renderer.ts
// Description: WebGL renderer policy: attached once per session after its first adopt and kept while parked; DOM renderer on context loss or failure

import type { Terminal } from "@xterm/xterm";
import { WebglAddon } from "@xterm/addon-webgl";
import { errorText } from "./terminal_session_io.js";

export interface RendererHandle {
  /** Drops the WebGL renderer; xterm reinstates its DOM renderer. Idempotent. */
  dispose(): void;
}

/**
 * Loads the WebGL addon on an opened, visible terminal. When the context is lost the addon is
 * disposed (xterm falls back to the DOM renderer) and `onLost` lets the owner attach again on
 * its next adopt. Sessions never exceed the app cap, so one context per session stays well
 * inside the browser's context limit.
 */
export function attachWebglRenderer(terminal: Terminal, onLost: () => void): RendererHandle {
  let addon: WebglAddon | null = new WebglAddon();
  const drop = (): void => {
    const current = addon;
    addon = null;
    current?.dispose();
  };
  addon.onContextLoss(() => {
    drop();
    onLost();
  });
  try {
    terminal.loadAddon(addon);
  } catch (error: unknown) {
    drop();
    console.warn(`[terminal] WebGL renderer unavailable, using the DOM renderer: ${errorText(error)}`);
  }
  return { dispose: drop };
}
