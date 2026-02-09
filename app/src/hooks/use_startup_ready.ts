// Path: app/src/hooks/use_startup_ready.ts
// Description: One-shot startup handshake to reveal main window after config load

import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

let didSignalStartupReady = false;
const RETRY_DELAY_MS_BASE = 250;
const RETRY_DELAY_MS_MAX = 5_000;

/**
 * Signals backend once the frontend shell is ready.
 * Uses module-level latch so StrictMode remount in dev does not double-invoke.
 * On success, sets data-boot-phase="ready" to trigger the CSS fade-in.
 */
export function useStartupReady(isLoaded: boolean): void {
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    if (!isLoaded || didSignalStartupReady) {
      return;
    }

    didSignalStartupReady = true;
    invoke("startup_ready")
      .then(() => {
        document.documentElement.dataset.bootPhase = "ready";
      })
      .catch((err: unknown) => {
        didSignalStartupReady = false;
        const delayMs = Math.min(
          RETRY_DELAY_MS_MAX,
          RETRY_DELAY_MS_BASE * 2 ** Math.min(attempt, 4)
        );
        console.error(
          `[startup] Failed to signal startup_ready (retry in ${delayMs}ms):`,
          err
        );
        window.setTimeout(() => {
          setAttempt((prev) => prev + 1);
        }, delayMs);
      });
  }, [isLoaded, attempt]);
}
