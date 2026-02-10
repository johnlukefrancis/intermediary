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
 * - preferred standard is always standard
 * - preferred handset can temporarily render standard on maximize/wide windows
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

    if (preferredUiMode === "standard") {
      setEffectiveUiMode("standard");
      return;
    }

    let mounted = true;
    let unlistenResize: (() => void) | null = null;

    const updateEffectiveMode = async (
      appWindow: ReturnType<typeof getCurrentWindow>
    ): Promise<void> => {
      const snapshot = await readWindowModeSnapshot(appWindow);
      if (!mounted) {
        return;
      }

      setEffectiveUiMode((previous) =>
        resolveEffectiveUiMode({
          preferredMode: preferredUiMode,
          width: snapshot.width,
          maximized: snapshot.maximized,
          previousEffectiveMode: previous,
        })
      );
    };

    const setup = async (): Promise<void> => {
      try {
        const appWindow = getCurrentWindow();
        // Reset baseline on handset preference so hysteresis starts in handset.
        setEffectiveUiMode("handset");
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
        await updateEffectiveMode(appWindow);
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
