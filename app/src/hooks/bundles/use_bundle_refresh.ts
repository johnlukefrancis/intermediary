// Path: app/src/hooks/bundles/use_bundle_refresh.ts
// Description: Bundle list refresh flow with transient WSL retry handling

import { useCallback, type Dispatch, type MutableRefObject, type SetStateAction } from "react";
import type { AgentClient } from "../../lib/agent/agent_client.js";
import type { ConnectionState } from "../../lib/agent/connection_state.js";
import { isStagingNotConfiguredError } from "../../lib/agent/error_codes.js";
import { sendListBundles } from "../../lib/agent/messages.js";
import {
  computeTransientRetryDelayMs,
  isTransientWslTransportError,
} from "../../lib/agent/transient_wsl_error.js";
import type { HelloState } from "../use_client_hello.js";
import type { BundlePresetState } from "./bundle_state_types.js";

interface UseBundleRefreshOptions {
  client: AgentClient | null;
  connectionStatus: ConnectionState["status"];
  helloStatus: HelloState["status"];
  repoId: string;
  resyncClientHello: () => Promise<boolean>;
  setPresets: Dispatch<SetStateAction<Map<string, BundlePresetState>>>;
  refreshRetryTimersRef: MutableRefObject<Map<string, ReturnType<typeof setTimeout>>>;
  refreshRetryAttemptsRef: MutableRefObject<Map<string, number>>;
  refreshInFlightRef: MutableRefObject<Set<string>>;
  refreshEpochRef: MutableRefObject<number>;
}

export function useBundleRefresh({
  client,
  connectionStatus,
  helloStatus,
  repoId,
  resyncClientHello,
  setPresets,
  refreshRetryTimersRef,
  refreshRetryAttemptsRef,
  refreshInFlightRef,
  refreshEpochRef,
}: UseBundleRefreshOptions): {
  clearRefreshRetry: (presetId: string) => void;
  clearAllRefreshRetries: () => void;
  refreshBundles: (presetId: string) => Promise<void>;
} {
  const clearRefreshRetry = useCallback((presetId: string) => {
    const timer = refreshRetryTimersRef.current.get(presetId);
    if (timer) {
      clearTimeout(timer);
      refreshRetryTimersRef.current.delete(presetId);
    }
    refreshRetryAttemptsRef.current.delete(presetId);
  }, [refreshRetryAttemptsRef, refreshRetryTimersRef]);

  const clearAllRefreshRetries = useCallback(() => {
    for (const timer of refreshRetryTimersRef.current.values()) {
      clearTimeout(timer);
    }
    refreshRetryTimersRef.current.clear();
    refreshRetryAttemptsRef.current.clear();
    refreshInFlightRef.current.clear();
  }, [refreshInFlightRef, refreshRetryAttemptsRef, refreshRetryTimersRef]);

  const refreshBundles = useCallback(
    async (presetId: string) => {
      if (!client || connectionStatus !== "connected" || helloStatus !== "ok") {
        return;
      }
      if (refreshInFlightRef.current.has(presetId)) return;
      refreshInFlightRef.current.add(presetId);
      const refreshEpoch = refreshEpochRef.current;

      try {
        const result = await sendListBundles(client, repoId, presetId);
        if (refreshEpoch !== refreshEpochRef.current) return;
        clearRefreshRetry(presetId);
        setPresets((prev) => {
          const next = new Map(prev);
          const preset = next.get(presetId);
          if (preset) {
            next.set(presetId, {
              ...preset,
              bundles: result.bundles,
              isBuilding: false,
              isCancelling: false,
              activeBuildId: null,
              buildProgress: null,
            });
          }
          return next;
        });
      } catch (err) {
        if (refreshEpoch !== refreshEpochRef.current) return;
        let errorForHandling: unknown = err;
        if (isStagingNotConfiguredError(err)) {
          const resynced = await resyncClientHello();
          if (refreshEpoch !== refreshEpochRef.current) return;
          if (resynced) {
            try {
              const retry = await sendListBundles(client, repoId, presetId);
              if (refreshEpoch !== refreshEpochRef.current) return;
              setPresets((prev) => {
                const next = new Map(prev);
                const preset = next.get(presetId);
                if (preset) {
                  next.set(presetId, {
                    ...preset,
                    bundles: retry.bundles,
                    isBuilding: false,
                    isCancelling: false,
                    activeBuildId: null,
                    buildProgress: null,
                    lastBuildError: null,
                  });
                }
                return next;
              });
              clearRefreshRetry(presetId);
              return;
            } catch (retryErr) {
              if (refreshEpoch !== refreshEpochRef.current) return;
              errorForHandling = retryErr;
            }
          }
        }
        if (isTransientWslTransportError(errorForHandling)) {
          const attempts = refreshRetryAttemptsRef.current.get(presetId) ?? 0;
          const delay = computeTransientRetryDelayMs(attempts);
          refreshRetryAttemptsRef.current.set(presetId, attempts + 1);
          const priorTimer = refreshRetryTimersRef.current.get(presetId);
          if (priorTimer) clearTimeout(priorTimer);
          const timer = setTimeout(() => {
            if (refreshEpoch !== refreshEpochRef.current) return;
            void refreshBundles(presetId);
          }, delay);
          refreshRetryTimersRef.current.set(presetId, timer);
          return;
        }
        clearRefreshRetry(presetId);
        console.error("[useBundleState] refreshBundles failed:", errorForHandling);
      } finally {
        refreshInFlightRef.current.delete(presetId);
      }
    },
    [
      clearRefreshRetry,
      client,
      connectionStatus,
      helloStatus,
      refreshEpochRef,
      refreshInFlightRef,
      refreshRetryAttemptsRef,
      refreshRetryTimersRef,
      repoId,
      resyncClientHello,
      setPresets,
    ]
  );

  return { clearRefreshRetry, clearAllRefreshRetries, refreshBundles };
}
