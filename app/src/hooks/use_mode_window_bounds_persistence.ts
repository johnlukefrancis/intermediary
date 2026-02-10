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
  const appWindowRef = useRef<ReturnType<typeof getCurrentWindow> | null>(null);
  const activeModeRef = useRef<UiMode>(uiMode);
  const mountedRef = useRef(false);
  const setWindowBoundsForModeRef = useRef(setWindowBoundsForMode);
  const saveTimeoutRef = useRef<number | null>(null);
  const lastSavedByModeRef = useRef<Partial<Record<UiMode, UiWindowBounds>>>({});

  setWindowBoundsForModeRef.current = setWindowBoundsForMode;

  const scheduleSave = (mode: UiMode, bounds: UiWindowBounds): void => {
    if (saveTimeoutRef.current !== null) {
      clearTimeout(saveTimeoutRef.current);
    }

    saveTimeoutRef.current = window.setTimeout(() => {
      if (!mountedRef.current) return;
      const lastSavedForMode = lastSavedByModeRef.current[mode];
      if (lastSavedForMode && areWindowBoundsEqual(lastSavedForMode, bounds)) {
        return;
      }
      setWindowBoundsForModeRef.current(mode, bounds);
      lastSavedByModeRef.current[mode] = bounds;
    }, SAVE_DEBOUNCE_MS);
  };

  const persistCurrentBounds = async (mode: UiMode): Promise<void> => {
    const appWindow = appWindowRef.current ?? getCurrentWindow();
    appWindowRef.current = appWindow;

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
    scheduleSave(mode, bounds);
  };

  useEffect(() => {
    mountedRef.current = true;
    let unlistenResize: (() => void) | null = null;

    const setup = async (): Promise<void> => {
      try {
        const appWindow = appWindowRef.current ?? getCurrentWindow();
        appWindowRef.current = appWindow;
        const unlisten = await appWindow.onResized(() => {
          void persistCurrentBounds(activeModeRef.current).catch((error: unknown) => {
            console.warn("[useModeWindowBoundsPersistence] resize persistence skipped:", error);
          });
        });
        if (!mountedRef.current) {
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
      mountedRef.current = false;
      if (saveTimeoutRef.current !== null) {
        clearTimeout(saveTimeoutRef.current);
      }
      unlistenResize?.();
      appWindowRef.current = null;
    };
  }, []);

  useEffect(() => {
    activeModeRef.current = uiMode;
    void persistCurrentBounds(uiMode).catch((error: unknown) => {
      console.warn("[useModeWindowBoundsPersistence] mode-change persistence skipped:", error);
    });
  }, [uiMode]);
}
