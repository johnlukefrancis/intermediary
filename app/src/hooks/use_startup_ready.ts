// Path: app/src/hooks/use_startup_ready.ts
// Description: One-shot startup handshake to reveal main window after config load

import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

let didSignalStartupReady = false;
const MAX_STARTUP_READY_ATTEMPTS = 20;

/**
 * Signals backend once the frontend shell is ready.
 * Uses module-level latch so StrictMode remount in dev does not double-invoke.
 * On success, sets data-boot-phase="ready" to trigger the CSS fade-in.
 */
export function useStartupReady(isLoaded: boolean): void {
  const [attempt, setAttempt] = useState(0);
  const [gaveUp, setGaveUp] = useState(false);

  useEffect(() => {
    if (!isLoaded || didSignalStartupReady || gaveUp) {
      return;
    }

    didSignalStartupReady = true;
    invoke("startup_ready")
      .then(() => {
        document.documentElement.dataset.bootPhase = "ready";
      })
      .catch((err: unknown) => {
        didSignalStartupReady = false;
        if (attempt + 1 >= MAX_STARTUP_READY_ATTEMPTS) {
          setGaveUp(true);
          document.documentElement.dataset.bootPhase = "ready";
          console.error("[startup] startup_ready failed after max retries:", err);
          return;
        }
        setAttempt((prev) => prev + 1);
        console.error("[startup] Failed to signal startup_ready:", err);
      });
  }, [isLoaded, attempt, gaveUp]);
}
