// Path: app/src/hooks/use_startup_ready.ts
// Description: One-shot startup handshake to reveal main window after config load

import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

let didSignalStartupReady = false;

/**
 * Signals backend once the frontend shell is ready.
 * Uses module-level latch so StrictMode remount in dev does not double-invoke.
 */
export function useStartupReady(isLoaded: boolean): void {
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    if (!isLoaded || didSignalStartupReady) {
      return;
    }

    didSignalStartupReady = true;
    void invoke("startup_ready").catch((err: unknown) => {
      didSignalStartupReady = false;
      setAttempt((prev) => prev + 1);
      console.error("[startup] Failed to signal startup_ready:", err);
    });
  }, [isLoaded, attempt]);
}
