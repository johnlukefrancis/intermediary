// Path: app/src/hooks/bundles/use_bundle_build_actions.ts
// Description: Build and cancel actions for bundle presets

import { useCallback, useRef, type Dispatch, type SetStateAction } from "react";
import type { AgentClient } from "../../lib/agent/agent_client.js";
import type { ConnectionState } from "../../lib/agent/connection_state.js";
import { isStagingNotConfiguredError } from "../../lib/agent/error_codes.js";
import { sendBuildBundle, sendCancelBundleBuild } from "../../lib/agent/messages.js";
import type { GlobalExcludes } from "../../shared/config.js";
import type { HelloState } from "../use_client_hello.js";
import type { BundlePresetState } from "./bundle_state_types.js";

const BUNDLE_BUILD_CANCELLED_CODE = "BUNDLE_BUILD_CANCELLED:";

interface UseBundleBuildActionsOptions {
  client: AgentClient | null;
  connectionStatus: ConnectionState["status"];
  helloStatus: HelloState["status"];
  globalExcludes: GlobalExcludes;
  presets: Map<string, BundlePresetState>;
  repoId: string;
  resyncClientHello: () => Promise<boolean>;
  refreshBundles: (presetId: string) => Promise<void>;
  setPresets: Dispatch<SetStateAction<Map<string, BundlePresetState>>>;
}

function createBundleBuildId(): string {
  if (typeof globalThis.crypto.randomUUID === "function") {
    return globalThis.crypto.randomUUID();
  }
  return `build_${Date.now()}_${Math.random().toString(16).slice(2)}`;
}

function isBundleBuildCancelledError(error: unknown): boolean {
  return error instanceof Error && error.message.startsWith(BUNDLE_BUILD_CANCELLED_CODE);
}

export function useBundleBuildActions({
  client,
  connectionStatus,
  helloStatus,
  globalExcludes,
  presets,
  repoId,
  resyncClientHello,
  refreshBundles,
  setPresets,
}: UseBundleBuildActionsOptions): {
  buildBundle: (presetId: string) => Promise<void>;
  cancelBundleBuild: (presetId: string) => Promise<void>;
} {
  const locallyCancelledBuildIdsRef = useRef<Set<string>>(new Set());

  const clearLocalBuildState = useCallback((presetId: string, buildId: string) => {
    setPresets((prev) => {
      const next = new Map(prev);
      const p = next.get(presetId);
      if (p && p.activeBuildId === buildId) {
        next.set(presetId, {
          ...p,
          isBuilding: false,
          isCancelling: false,
          activeBuildId: null,
          buildProgress: null,
          lastBuildError: null,
        });
      }
      return next;
    });
  }, [setPresets]);

  const buildBundle = useCallback(
    async (presetId: string) => {
      if (!client || connectionStatus !== "connected") return;

      const preset = presets.get(presetId);
      if (!preset || preset.isBuilding) return;
      const buildId = createBundleBuildId();
      if (helloStatus !== "ok") {
        setPresets((prev) => {
          const next = new Map(prev);
          const p = next.get(presetId);
          if (p) {
            next.set(presetId, {
              ...p,
              isBuilding: false,
              isCancelling: false,
              activeBuildId: null,
              buildProgress: null,
              lastBuildError: "Agent session initializing; retry in a moment.",
            });
          }
          return next;
        });
        return;
      }

      setPresets((prev) => {
        const next = new Map(prev);
        const p = next.get(presetId);
        if (p) {
          next.set(presetId, {
            ...p,
            isBuilding: true,
            isCancelling: false,
            activeBuildId: buildId,
            buildProgress: { phase: "scanning", filesDone: 0, filesTotal: 0 },
            lastBuildError: null,
          });
        }
        return next;
      });

      try {
        try {
          await sendBuildBundle(client, repoId, presetId, buildId, preset.selection, globalExcludes);
        } catch (err) {
          if (!isStagingNotConfiguredError(err)) {
            throw err;
          }
          const resynced = await resyncClientHello();
          if (!resynced) {
            throw err;
          }
          if (locallyCancelledBuildIdsRef.current.has(buildId)) {
            clearLocalBuildState(presetId, buildId);
            void refreshBundles(presetId);
            return;
          }
          await sendBuildBundle(client, repoId, presetId, buildId, preset.selection, globalExcludes);
        }
        setPresets((prev) => {
          const next = new Map(prev);
          const p = next.get(presetId);
          if (p && p.activeBuildId === buildId) {
            next.set(presetId, {
              ...p,
              isBuilding: false,
              isCancelling: false,
              activeBuildId: null,
              buildProgress: null,
              freshlyBuiltAt: Date.now(),
            });
          }
          return next;
        });
        void refreshBundles(presetId);
      } catch (err) {
        const isCancelled = isBundleBuildCancelledError(err) || locallyCancelledBuildIdsRef.current.has(buildId);
        const message = err instanceof Error ? err.message : String(err);
        setPresets((prev) => {
          const next = new Map(prev);
          const p = next.get(presetId);
          if (p && p.activeBuildId === buildId) {
            next.set(presetId, {
              ...p,
              isBuilding: false,
              isCancelling: false,
              activeBuildId: null,
              buildProgress: null,
              lastBuildError: isCancelled ? null : message,
            });
          }
          return next;
        });
        if (isCancelled) {
          void refreshBundles(presetId);
        }
      } finally {
        locallyCancelledBuildIdsRef.current.delete(buildId);
      }
    },
    [
      client,
      clearLocalBuildState,
      connectionStatus,
      globalExcludes,
      helloStatus,
      presets,
      refreshBundles,
      repoId,
      resyncClientHello,
      setPresets,
    ]
  );

  const cancelBundleBuild = useCallback(
    async (presetId: string) => {
      if (!client || connectionStatus !== "connected") return;

      const preset = presets.get(presetId);
      const buildId = preset?.activeBuildId;
      if (!preset?.isBuilding || !buildId || preset.isCancelling) return;
      locallyCancelledBuildIdsRef.current.add(buildId);

      setPresets((prev) => {
        const next = new Map(prev);
        const p = next.get(presetId);
        if (p && p.activeBuildId === buildId) {
          next.set(presetId, { ...p, isCancelling: true, lastBuildError: null });
        }
        return next;
      });

      try {
        const result = await sendCancelBundleBuild(client, repoId, presetId, buildId);
        if (!result.cancelled) {
          return;
        }
      } catch (err) {
        locallyCancelledBuildIdsRef.current.delete(buildId);
        const message = err instanceof Error ? err.message : String(err);
        setPresets((prev) => {
          const next = new Map(prev);
          const p = next.get(presetId);
          if (p && p.activeBuildId === buildId) {
            next.set(presetId, { ...p, isCancelling: false, lastBuildError: message });
          }
          return next;
        });
      }
    },
    [client, connectionStatus, presets, repoId, setPresets]
  );

  return { buildBundle, cancelBundleBuild };
}
