// Path: app/src/hooks/use_motion_governor.ts
// Description: Pauses motion when window is hidden/minimized to save GPU

import { useState, useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

export interface UseMotionGovernorResult {
  motionPaused: boolean;
}

/**
 * Detects when the window is hidden (minimized/occluded) and signals
 * that animations should be paused to reduce background GPU usage.
 *
 * Uses document.visibilitychange as primary signal, with Tauri window
 * focus events as secondary signal for Windows edge cases.
 */
export function useMotionGovernor(): UseMotionGovernorResult {
  const [motionPaused, setMotionPaused] = useState<boolean>(() => document.hidden);

  useEffect(() => {
    // Primary: document visibility change (reliable cross-platform)
    const handleVisibilityChange = (): void => {
      setMotionPaused(document.hidden);
    };
    document.addEventListener("visibilitychange", handleVisibilityChange);

    // Secondary: Tauri window focus/minimize detection
    // On Windows, visibilitychange may not fire reliably on minimize
    let unlistenFocus: (() => void) | null = null;
    let mounted = true;

    const setupTauriListener = async (): Promise<void> => {
      try {
        const appWindow = getCurrentWindow();
        const unlisten = await appWindow.onFocusChanged(({ payload: focused }) => {
          if (!mounted) return;

          if (!focused) {
            // Lost focus - check if minimized
            void appWindow.isMinimized().then((minimized) => {
              if (mounted && minimized) {
                setMotionPaused(true);
              }
            });
          } else {
            // Gained focus - always unpause (window is visible)
            setMotionPaused(false);
          }
        });

        if (mounted) {
          unlistenFocus = unlisten;
        } else {
          unlisten();
        }
      } catch {
        // Tauri APIs may not be available in dev/test environments
        // Document visibility is sufficient fallback
      }
    };

    void setupTauriListener();

    return () => {
      mounted = false;
      document.removeEventListener("visibilitychange", handleVisibilityChange);
      unlistenFocus?.();
    };
  }, []);

  return { motionPaused };
}
