// Path: app/src/hooks/use_handset_window_snap.ts
// Description: Snap main window geometry when toggling handset mode and restore on exit

import { useEffect, useRef } from "react";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { UiMode } from "../shared/config.js";

const HANDSET_WINDOW_WIDTH = 420;
const HANDSET_WINDOW_HEIGHT = 720;

interface WindowSnapshot {
  width: number;
  height: number;
  wasMaximized: boolean;
}

/**
 * Best-effort window geometry management for handset mode.
 * - Enter handset: capture current logical size/state, unmaximize if needed, then snap.
 * - Exit handset: restore previous maximized state or logical size.
 */
export function useHandsetWindowSnap(uiMode: UiMode): void {
  const previousModeRef = useRef<UiMode | null>(null);
  const snapshotRef = useRef<WindowSnapshot | null>(null);

  useEffect(() => {
    const previousMode = previousModeRef.current;
    const enteringHandset = uiMode === "handset" && previousMode !== "handset";
    const leavingHandset = previousMode === "handset" && uiMode !== "handset";

    previousModeRef.current = uiMode;

    if (!enteringHandset && !leavingHandset) {
      return;
    }

    const run = async (): Promise<void> => {
      try {
        const appWindow = getCurrentWindow();
        const fullscreen = await appWindow.isFullscreen();
        if (fullscreen) return;

        if (enteringHandset) {
          const resizable = await appWindow.isResizable();
          if (!resizable) return;

          const wasMaximized = await appWindow.isMaximized();

          if (wasMaximized) {
            await appWindow.unmaximize();
          }

          const size = await appWindow.innerSize();
          const scaleFactor = await appWindow.scaleFactor();

          const logicalSize = size.toLogical(scaleFactor);
          snapshotRef.current = {
            width: logicalSize.width,
            height: logicalSize.height,
            wasMaximized,
          };

          await appWindow.setSize(
            new LogicalSize(HANDSET_WINDOW_WIDTH, HANDSET_WINDOW_HEIGHT)
          );
          return;
        }

        if (!leavingHandset) return;

        const snapshot = snapshotRef.current;
        snapshotRef.current = null;
        if (!snapshot) return;

        if (snapshot.wasMaximized) {
          const maximizable = await appWindow.isMaximizable();
          if (!maximizable) return;
          await appWindow.maximize();
          return;
        }

        const resizable = await appWindow.isResizable();
        if (!resizable) return;

        await appWindow.setSize(new LogicalSize(snapshot.width, snapshot.height));
      } catch (error) {
        // Tauri APIs can be unavailable in browser-only contexts.
        console.warn("[useHandsetWindowSnap] window geometry update skipped:", error);
      }
    };

    void run();
  }, [uiMode]);
}
