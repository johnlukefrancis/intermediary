// Path: app/src/hooks/use_mode_window_snap.ts
// Description: Applies per-mode window bounds when the active UI mode changes

import { useEffect, useRef } from "react";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { UiMode, UiWindowBoundsByMode } from "../shared/config.js";
import { resolveModeWindowBounds } from "../lib/window/mode_window_bounds.js";

/**
 * Best-effort window geometry management by UI mode.
 * Applies configured/default bounds when mode changes (including initial mount).
 */
export function useModeWindowSnap(
  uiMode: UiMode,
  windowBoundsByMode: UiWindowBoundsByMode
): void {
  const previousModeRef = useRef<UiMode | null>(null);

  useEffect(() => {
    const previousMode = previousModeRef.current;
    previousModeRef.current = uiMode;

    if (previousMode === uiMode) {
      return;
    }

    const run = async (): Promise<void> => {
      try {
        const appWindow = getCurrentWindow();
        const fullscreen = await appWindow.isFullscreen();
        if (fullscreen) return;

        const resizable = await appWindow.isResizable();
        if (!resizable) return;

        const maximized = await appWindow.isMaximized();
        if (maximized) {
          await appWindow.unmaximize();
        }

        const bounds = resolveModeWindowBounds(uiMode, windowBoundsByMode);
        await appWindow.setSize(new LogicalSize(bounds.width, bounds.height));
      } catch (error) {
        // Tauri APIs can be unavailable in browser-only contexts.
        console.warn("[useModeWindowSnap] window geometry update skipped:", error);
      }
    };

    void run();
  }, [uiMode, windowBoundsByMode]);
}
