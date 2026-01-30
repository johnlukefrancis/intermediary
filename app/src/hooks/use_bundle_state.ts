// Path: app/src/hooks/use_bundle_state.ts
// Description: Per-repo bundle state management with event subscription

import { useState, useEffect, useCallback, useRef } from "react";
import { useAgent } from "./use_agent.js";
import { sendBuildBundle, sendListBundles } from "../lib/agent/messages.js";
import type {
  BundleInfo,
  BundleSelection,
  AgentEvent,
} from "../shared/protocol.js";
import { DEFAULT_BUNDLE_PRESET } from "../shared/config.js";

export interface BundlePresetState {
  presetId: string;
  presetName: string;
  selection: BundleSelection;
  isBuilding: boolean;
  bundles: BundleInfo[];
  lastBuildError: string | null;
}

export interface BundleState {
  presets: Map<string, BundlePresetState>;
  activePresetId: string;
  topLevelDirs: string[];
  setSelection: (presetId: string, selection: BundleSelection) => void;
  buildBundle: (presetId: string) => Promise<void>;
  setActivePreset: (presetId: string) => void;
  refreshBundles: (presetId: string) => Promise<void>;
}

function createDefaultPresetState(): BundlePresetState {
  return {
    presetId: DEFAULT_BUNDLE_PRESET.presetId,
    presetName: DEFAULT_BUNDLE_PRESET.presetName,
    selection: {
      includeRoot: DEFAULT_BUNDLE_PRESET.includeRoot,
      // Empty means ALL - but we track actual dirs for UI display
      topLevelDirs: [],
    },
    isBuilding: false,
    bundles: [],
    lastBuildError: null,
  };
}

export function useBundleState(repoId: string, topLevelDirs: string[]): BundleState {
  const { subscribe, client, connectionState, helloState } = useAgent();

  const [presets, setPresets] = useState<Map<string, BundlePresetState>>(
    () => new Map([[DEFAULT_BUNDLE_PRESET.presetId, createDefaultPresetState()]])
  );
  const [activePresetId, setActivePresetId] = useState(DEFAULT_BUNDLE_PRESET.presetId);
  const lastRefreshKeyRef = useRef<string | null>(null);

  // Update selection for a preset
  const setSelection = useCallback((presetId: string, selection: BundleSelection) => {
    setPresets((prev) => {
      const next = new Map(prev);
      const preset = next.get(presetId);
      if (preset) {
        next.set(presetId, { ...preset, selection, lastBuildError: null });
      }
      return next;
    });
  }, []);

  // Build bundle for a preset
  const buildBundle = useCallback(
    async (presetId: string) => {
      if (!client || connectionState.status !== "connected") return;

      const preset = presets.get(presetId);
      if (!preset || preset.isBuilding) return;

      // Mark as building
      setPresets((prev) => {
        const next = new Map(prev);
        const p = next.get(presetId);
        if (p) {
          next.set(presetId, { ...p, isBuilding: true, lastBuildError: null });
        }
        return next;
      });

      try {
        await sendBuildBundle(client, repoId, presetId, preset.selection);
        // Bundle list will be refreshed via bundleBuilt event
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        setPresets((prev) => {
          const next = new Map(prev);
          const p = next.get(presetId);
          if (p) {
            next.set(presetId, { ...p, isBuilding: false, lastBuildError: message });
          }
          return next;
        });
      }
    },
    [client, connectionState.status, presets, repoId]
  );

  // Refresh bundle list for a preset
  const refreshBundles = useCallback(
    async (presetId: string) => {
      if (!client || connectionState.status !== "connected") return;

      try {
        const result = await sendListBundles(client, repoId, presetId);
        setPresets((prev) => {
          const next = new Map(prev);
          const preset = next.get(presetId);
          if (preset) {
            next.set(presetId, { ...preset, bundles: result.bundles, isBuilding: false });
          }
          return next;
        });
      } catch (err) {
        console.error("[useBundleState] refreshBundles failed:", err);
      }
    },
    [client, connectionState.status, repoId]
  );

  // Handle agent events
  const handleEvent = useCallback(
    (event: AgentEvent) => {
      if (event.type === "bundleBuilt" && event.repoId === repoId) {
        // Refresh bundle list after build
        void refreshBundles(event.presetId);
      }
    },
    [repoId, refreshBundles]
  );

  // Subscribe to events
  useEffect(() => {
    const unsubscribe = subscribe(handleEvent);
    return unsubscribe;
  }, [subscribe, handleEvent]);

  // Reset state when repoId changes
  useEffect(() => {
    setPresets(new Map([[DEFAULT_BUNDLE_PRESET.presetId, createDefaultPresetState()]]));
    setActivePresetId(DEFAULT_BUNDLE_PRESET.presetId);
    lastRefreshKeyRef.current = null;
  }, [repoId]);

  // Refresh bundles on connect
  useEffect(() => {
    if (
      connectionState.status !== "connected" ||
      !client ||
      helloState.status !== "ok" ||
      helloState.lastHelloAt === null
    ) {
      return;
    }
    if (!helloState.watchedRepoIds.includes(repoId)) {
      return;
    }

    const refreshKey = `${repoId}:${activePresetId}:${helloState.lastHelloAt}`;
    if (lastRefreshKeyRef.current === refreshKey) {
      return;
    }
    lastRefreshKeyRef.current = refreshKey;

    void refreshBundles(activePresetId);
  }, [
    repoId,
    activePresetId,
    client,
    connectionState.status,
    helloState.status,
    helloState.lastHelloAt,
    helloState.watchedRepoIds,
    refreshBundles,
  ]);

  return {
    presets,
    activePresetId,
    topLevelDirs,
    setSelection,
    buildBundle,
    setActivePreset: setActivePresetId,
    refreshBundles,
  };
}
