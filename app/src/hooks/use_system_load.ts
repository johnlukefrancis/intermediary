// Path: app/src/hooks/use_system_load.ts
// Description: Owns the system-load sampling cadence: one request 1 s after the last resolves, only while visible

import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { SystemLoadSnapshotSchema } from "../shared/protocol_system_load.js";
import { advanceLoadState, type LoadState } from "../lib/system_load/load_state.js";

const SAMPLE_GAP_MS = 1000;
const RETRY_GAP_MS = 10_000;

/** Null until the first rated sample and while sampling fails; failures retry every 10 s. */
export function useSystemLoad(documentHidden: boolean): LoadState | null {
  const [state, setState] = useState<LoadState | null>(null);

  useEffect(() => {
    if (documentHidden) return;
    let cancelled = false;
    let failing = false;
    let timer: number | undefined;
    const request = (): void => {
      invoke<unknown>("sample_system_load")
        .then((raw) => {
          if (cancelled) return;
          const snapshot = SystemLoadSnapshotSchema.parse(raw);
          failing = false;
          // The sampler's first sample has no interval, so every rate in it reads 0.
          if (snapshot.intervalMs > 0) setState((previous) => advanceLoadState(previous, snapshot));
          timer = window.setTimeout(request, SAMPLE_GAP_MS);
        })
        .catch((err: unknown) => {
          if (cancelled) return;
          if (!failing) console.warn("[system-load] Sampling failed; hiding meters:", err);
          failing = true;
          setState(null);
          timer = window.setTimeout(request, RETRY_GAP_MS);
        });
    };
    request();
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [documentHidden]);

  return state;
}
