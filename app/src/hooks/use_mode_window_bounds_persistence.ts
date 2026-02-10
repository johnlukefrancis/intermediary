// Path: app/src/hooks/use_mode_window_bounds_persistence.ts
// Description: Persists window bounds per mode from live resize events

import { useEffect, useRef } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { UiMode, UiWindowBounds } from "../shared/config.js";
import {
  areWindowBoundsEqual,
  clampWindowBounds,
} from "../lib/window/mode_window_bounds.js";

const SAVE_DEBOUNCE_MS = 300;

/**
 * Listen to window resize and persist logical bounds under the current mode.
 */
export function useModeWindowBoundsPersistence(
  uiMode: UiMode,
  setWindowBoundsForMode: (mode: UiMode, bounds: UiWindowBounds) => void
): void {
  const saveTimeoutRef = useRef<number | null>(null);
  const lastSavedRef = useRef<UiWindowBounds | null>(null);

  useEffect(() => {
    let mounted = true;
    let unlistenResize: (() => void) | null = null;

    const scheduleSave = (bounds: UiWindowBounds): void => {
      if (saveTimeoutRef.current !== null) {
        clearTimeout(saveTimeoutRef.current);
      }

      saveTimeoutRef.current = window.setTimeout(() => {
        if (!mounted) return;
        if (lastSavedRef.current && areWindowBoundsEqual(lastSavedRef.current, bounds)) {
          return;
        }
        setWindowBoundsForMode(uiMode, bounds);
        lastSavedRef.current = bounds;
      }, SAVE_DEBOUNCE_MS);
    };

    const persistCurrentBounds = async (): Promise<void> => {
      const appWindow = getCurrentWindow();
      const fullscreen = await appWindow.isFullscreen();
      const maximized = await appWindow.isMaximized();
      if (fullscreen || maximized) {
        return;
      }

      const size = await appWindow.innerSize();
      const scaleFactor = await appWindow.scaleFactor();
      const logicalSize = size.toLogical(scaleFactor);
      const bounds = clampWindowBounds({
        width: logicalSize.width,
        height: logicalSize.height,
      });
      scheduleSave(bounds);
    };

    const setup = async (): Promise<void> => {
      try {
        const appWindow = getCurrentWindow();
        const unlisten = await appWindow.onResized(() => {
          void persistCurrentBounds().catch((error: unknown) => {
            console.warn("[useModeWindowBoundsPersistence] resize persistence skipped:", error);
          });
        });
        if (!mounted) {
          unlisten();
          return;
        }
        unlistenResize = unlisten;
      } catch (error) {
        // Tauri APIs can be unavailable in browser-only contexts.
        console.warn("[useModeWindowBoundsPersistence] listener setup skipped:", error);
      }
    };

    void setup();

    return () => {
      mounted = false;
      if (saveTimeoutRef.current !== null) {
        clearTimeout(saveTimeoutRef.current);
      }
      unlistenResize?.();
    };
  }, [uiMode, setWindowBoundsForMode]);
}
