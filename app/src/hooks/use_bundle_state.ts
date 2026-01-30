// Path: app/src/hooks/use_bundle_state.ts
// Description: Per-repo bundle state management with event subscription

import { useState, useEffect, useCallback, useMemo, useRef } from "react";
import { useAgent } from "./use_agent.js";
import { sendBuildBundle, sendListBundles } from "../lib/agent/messages.js";
import type {
  BundleInfo,
  BundleSelection,
  AgentEvent,
} from "../shared/protocol.js";
import { DEFAULT_BUNDLE_PRESET, type BundlePreset } from "../shared/config.js";

export interface BundlePresetState {
  presetId: string;
  presetName: string;
  selection: BundleSelection;
  isSelectionInitialized: boolean;
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

function normalizeTopLevelDirs(dirs: string[], available: string[] = []): string[] {
  const unique = Array.from(new Set(dirs.filter((dir) => dir.length > 0)));
  if (available.length === 0) {
    return unique.sort();
  }
  const allowed = new Set(available);
  return unique.filter((dir) => allowed.has(dir)).sort();
}

function createPresetState(
  preset: BundlePreset,
  topLevelDirs: string[] = []
): BundlePresetState {
  if (preset.topLevelDirs.length > 0) {
    return {
      presetId: preset.presetId,
      presetName: preset.presetName,
      selection: {
        includeRoot: preset.includeRoot,
        topLevelDirs: normalizeTopLevelDirs(preset.topLevelDirs, topLevelDirs),
      },
      isSelectionInitialized: true,
      isBuilding: false,
      bundles: [],
      lastBuildError: null,
    };
  }

  return {
    presetId: preset.presetId,
    presetName: preset.presetName,
    selection: {
      includeRoot: preset.includeRoot,
      topLevelDirs: topLevelDirs.length > 0 ? [...topLevelDirs].sort() : [],
    },
    isSelectionInitialized: topLevelDirs.length > 0,
    isBuilding: false,
    bundles: [],
    lastBuildError: null,
  };
}

function getRepoPresets(presets: BundlePreset[]): BundlePreset[] {
  if (presets.length > 0) {
    return presets;
  }
  return [
    {
      presetId: DEFAULT_BUNDLE_PRESET.presetId,
      presetName: DEFAULT_BUNDLE_PRESET.presetName,
      includeRoot: DEFAULT_BUNDLE_PRESET.includeRoot,
      topLevelDirs: DEFAULT_BUNDLE_PRESET.topLevelDirs,
    },
  ];
}

export function useBundleState(repoId: string, topLevelDirs: string[]): BundleState {
  const { subscribe, client, connectionState, helloState, config } = useAgent();

  const repoPresets = useMemo(() => {
    const repoConfig = config.repos.find((repo) => repo.repoId === repoId);
    return getRepoPresets(repoConfig?.bundlePresets ?? []);
  }, [config.repos, repoId]);

  const [presets, setPresets] = useState<Map<string, BundlePresetState>>(() => {
    const initial = new Map<string, BundlePresetState>();
    for (const preset of repoPresets) {
      initial.set(preset.presetId, createPresetState(preset, topLevelDirs));
    }
    return initial;
  });
  const [activePresetId, setActivePresetId] = useState(
    repoPresets[0]?.presetId ?? DEFAULT_BUNDLE_PRESET.presetId
  );
  const lastRefreshKeyRef = useRef<string | null>(null);

  // Update selection for a preset
  const setSelection = useCallback((presetId: string, selection: BundleSelection) => {
    setPresets((prev) => {
      const next = new Map(prev);
      const preset = next.get(presetId);
      if (preset) {
        next.set(presetId, {
          ...preset,
          selection,
          isSelectionInitialized: true,
          lastBuildError: null,
        });
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
    const next = new Map<string, BundlePresetState>();
    for (const preset of repoPresets) {
      next.set(preset.presetId, createPresetState(preset));
    }
    setPresets(next);
    setActivePresetId(repoPresets[0]?.presetId ?? DEFAULT_BUNDLE_PRESET.presetId);
    lastRefreshKeyRef.current = null;
  }, [repoId, repoPresets]);

  useEffect(() => {
    if (topLevelDirs.length === 0) {
      return;
    }
    setPresets((prev) => {
      let changed = false;
      const next = new Map(prev);
      for (const preset of next.values()) {
        if (!preset.isSelectionInitialized) {
          next.set(preset.presetId, {
            ...preset,
            selection: {
              includeRoot: preset.selection.includeRoot,
              topLevelDirs: [...topLevelDirs].sort(),
            },
            isSelectionInitialized: true,
          });
          changed = true;
        }
      }
      return changed ? next : prev;
    });
  }, [topLevelDirs]);

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
