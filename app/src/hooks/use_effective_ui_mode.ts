// Path: app/src/hooks/use_effective_ui_mode.ts
// Description: Derives runtime effective UI mode from preferred mode and live window state

import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { UiMode } from "../shared/config.js";
import { resolveEffectiveUiMode } from "../lib/window/effective_ui_mode_policy.js";

interface WindowModeSnapshot {
  width: number;
  maximized: boolean;
}

async function readWindowModeSnapshot(
  appWindow: ReturnType<typeof getCurrentWindow>
): Promise<WindowModeSnapshot> {
  const [maximized, size, scaleFactor] = await Promise.all([
    appWindow.isMaximized(),
    appWindow.innerSize(),
    appWindow.scaleFactor(),
  ]);
  const logicalSize = size.toLogical(scaleFactor);
  return {
    width: logicalSize.width,
    maximized,
  };
}

/**
 * Computes runtime mode used for rendering:
 * - selected mode is the baseline preference in config
 * - runtime mode can switch on maximize/width thresholds in either direction
 */
export function useEffectiveUiMode(
  preferredUiMode: UiMode,
  isLoaded: boolean
): UiMode {
  const [effectiveUiMode, setEffectiveUiMode] = useState<UiMode>(preferredUiMode);

  useEffect(() => {
    if (!isLoaded) {
      setEffectiveUiMode(preferredUiMode);
      return;
    }

    let mounted = true;
    let unlistenResize: (() => void) | null = null;

    const updateEffectiveMode = async (
      appWindow: ReturnType<typeof getCurrentWindow>,
      previousModeOverride?: UiMode
    ): Promise<void> => {
      const snapshot = await readWindowModeSnapshot(appWindow);
      if (!mounted) {
        return;
      }

      setEffectiveUiMode((previous) =>
        resolveEffectiveUiMode({
          width: snapshot.width,
          maximized: snapshot.maximized,
          previousEffectiveMode: previousModeOverride ?? previous,
        })
      );
    };

    const setup = async (): Promise<void> => {
      try {
        const appWindow = getCurrentWindow();
        // Reset baseline to selected preference so hysteresis starts from user intent.
        setEffectiveUiMode(preferredUiMode);
        const unlisten = await appWindow.onResized(() => {
          void updateEffectiveMode(appWindow).catch((error: unknown) => {
            console.warn("[useEffectiveUiMode] resize evaluation skipped:", error);
          });
        });
        if (!mounted) {
          unlisten();
          return;
        }
        unlistenResize = unlisten;
        await updateEffectiveMode(appWindow, preferredUiMode);
      } catch (error) {
        // Tauri APIs can be unavailable in browser-only contexts.
        if (mounted) {
          console.warn("[useEffectiveUiMode] runtime mode override unavailable:", error);
          setEffectiveUiMode(preferredUiMode);
        }
      }
    };

    void setup();

    return () => {
      mounted = false;
      unlistenResize?.();
    };
  }, [isLoaded, preferredUiMode]);

  return effectiveUiMode;
}
