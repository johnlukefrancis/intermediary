// Path: app/src/hooks/use_bundle_state.ts
// Description: Per-repo bundle state management with event subscription

import { useState, useEffect, useCallback, useMemo, useRef } from "react";
import { useAgent } from "./use_agent.js";
import { useConfig } from "./use_config.js";
import { sendBuildBundle, sendListBundles } from "../lib/agent/messages.js";
import type {
  BundleInfo,
  BundleSelection,
  AgentEvent,
  BundleBuildPhase,
} from "../shared/protocol.js";
import { DEFAULT_BUNDLE_PRESET, type BundlePreset } from "../shared/config.js";

interface BundleBuildProgress {
  phase: BundleBuildPhase;
  filesDone: number;
  filesTotal: number;
  currentFile?: string;
  currentBytesDone?: number;
  currentBytesTotal?: number;
  bytesDoneTotalBestEffort?: number;
}

export interface BundlePresetState {
  presetId: string;
  presetName: string;
  selection: BundleSelection;
  isSelectionInitialized: boolean;
  isBuilding: boolean;
  buildProgress: BundleBuildProgress | null;
  bundles: BundleInfo[];
  lastBuildError: string | null;
  /** Timestamp (ms) when bundle was last built, for fresh pulse animation */
  freshlyBuiltAt: number | null;
}

export interface BundleState {
  presets: Map<string, BundlePresetState>;
  activePresetId: string;
  topLevelDirs: string[];
  topLevelSubdirs: Record<string, string[]>;
  setSelection: (presetId: string, selection: BundleSelection) => void;
  buildBundle: (presetId: string) => Promise<void>;
  setActivePreset: (presetId: string) => void;
  refreshBundles: (presetId: string) => Promise<void>;
}

const EMPTY_SAVED_SELECTIONS: Record<string, BundleSelection> = {};

function buildPresetKey(presets: BundlePreset[]): string {
  return presets
    .map((preset) => {
      const dirs = preset.topLevelDirs.join(",");
      return `${preset.presetId}:${preset.presetName}:${preset.includeRoot}:${dirs}`;
    })
    .join("|");
}

function buildSelectionKey(selections: Record<string, BundleSelection>): string {
  const entries = Object.keys(selections)
    .sort()
    .map((presetId) => {
      const selection = selections[presetId];
      if (!selection) {
        return `${presetId}:missing`;
      }
      const dirs = selection.topLevelDirs.join(",");
      const excluded = selection.excludedSubdirs.join(",");
      return `${presetId}:${selection.includeRoot}:${dirs}:${excluded}`;
    });
  return entries.join("|");
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
  topLevelDirs: string[] = [],
  savedSelection?: BundleSelection
): BundlePresetState {
  // If we have a saved selection, use it
  if (savedSelection) {
    return {
      presetId: preset.presetId,
      presetName: preset.presetName,
      selection: {
        includeRoot: savedSelection.includeRoot,
        topLevelDirs: normalizeTopLevelDirs(savedSelection.topLevelDirs, topLevelDirs),
        excludedSubdirs: savedSelection.excludedSubdirs,
      },
      isSelectionInitialized: true,
      isBuilding: false,
      buildProgress: null,
      bundles: [],
      lastBuildError: null,
      freshlyBuiltAt: null,
    };
  }

  // If preset has explicit dirs, use those
  if (preset.topLevelDirs.length > 0) {
    return {
      presetId: preset.presetId,
      presetName: preset.presetName,
      selection: {
        includeRoot: preset.includeRoot,
        topLevelDirs: normalizeTopLevelDirs(preset.topLevelDirs, topLevelDirs),
        excludedSubdirs: [],
      },
      isSelectionInitialized: true,
      isBuilding: false,
      buildProgress: null,
      bundles: [],
      lastBuildError: null,
      freshlyBuiltAt: null,
    };
  }

  // Default to all available dirs
  return {
    presetId: preset.presetId,
    presetName: preset.presetName,
    selection: {
      includeRoot: preset.includeRoot,
      topLevelDirs: topLevelDirs.length > 0 ? [...topLevelDirs].sort() : [],
      excludedSubdirs: [],
    },
    isSelectionInitialized: topLevelDirs.length > 0,
    isBuilding: false,
    buildProgress: null,
    bundles: [],
    lastBuildError: null,
    freshlyBuiltAt: null,
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

export function useBundleState(
  repoId: string,
  topLevelDirs: string[],
  topLevelSubdirs: Record<string, string[]>
): BundleState {
  const { subscribe, client, connectionState, helloState, config } = useAgent();
  const { config: persistedConfig, setBundleSelection: persistSelection } = useConfig();

  // Get saved selections for this repo
  const savedSelections = useMemo(
    () => persistedConfig.bundleSelections[repoId] ?? EMPTY_SAVED_SELECTIONS,
    [persistedConfig.bundleSelections, repoId]
  );

  const repoPresets = useMemo(() => {
    const repoConfig = config.repos.find((repo) => repo.repoId === repoId);
    return getRepoPresets(repoConfig?.bundlePresets ?? []);
  }, [config.repos, repoId]);

  const [presets, setPresets] = useState<Map<string, BundlePresetState>>(() => {
    const initial = new Map<string, BundlePresetState>();
    for (const preset of repoPresets) {
      const saved = savedSelections[preset.presetId];
      initial.set(preset.presetId, createPresetState(preset, topLevelDirs, saved));
    }
    return initial;
  });
  const [activePresetId, setActivePresetId] = useState(
    repoPresets[0]?.presetId ?? DEFAULT_BUNDLE_PRESET.presetId
  );
  const lastRefreshKeyRef = useRef<string | null>(null);
  const resetKeyRef = useRef<string | null>(null);
  const lastProgressUpdateRef = useRef<
    Map<string, { ts: number; phase: BundleBuildPhase; filesDone: number; filesTotal: number; currentFile?: string }>
  >(
    new Map()
  );
  const PROGRESS_THROTTLE_MS = 500;

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
    // Persist to config
    persistSelection(repoId, presetId, selection);
  }, [repoId, persistSelection]);

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
            next.set(presetId, {
              ...preset,
              bundles: result.bundles,
              isBuilding: false,
              buildProgress: null,
            });
          }
          return next;
        });
      } catch (err) {
        console.error("[useBundleState] refreshBundles failed:", err);
      }
    },
    [client, connectionState.status, repoId]
  );

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
          next.set(presetId, {
            ...p,
            isBuilding: true,
            buildProgress: {
              phase: "scanning",
              filesDone: 0,
              filesTotal: 0,
            },
            lastBuildError: null,
          });
        }
        return next;
      });

      try {
        // Pass global excludes from persisted config
        const globalExcludes = persistedConfig.globalExcludes;
        await sendBuildBundle(client, repoId, presetId, preset.selection, globalExcludes);
        setPresets((prev) => {
          const next = new Map(prev);
          const p = next.get(presetId);
          if (p) {
            next.set(presetId, {
              ...p,
              isBuilding: false,
              buildProgress: null,
              freshlyBuiltAt: Date.now(),
            });
          }
          return next;
        });
        // Keep event-driven updates, but do not depend solely on event delivery for completion.
        void refreshBundles(presetId);
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        setPresets((prev) => {
          const next = new Map(prev);
          const p = next.get(presetId);
          if (p) {
            next.set(presetId, {
              ...p,
              isBuilding: false,
              buildProgress: null,
              lastBuildError: message,
            });
          }
          return next;
        });
      }
    },
    [client, connectionState.status, presets, repoId, persistedConfig.globalExcludes, refreshBundles]
  );

  // Handle agent events
  const handleEvent = useCallback(
    (event: AgentEvent) => {
      if (event.type === "bundleBuilt" && event.repoId === repoId) {
        setPresets((prev) => {
          const next = new Map(prev);
          const preset = next.get(event.presetId);
          if (preset) {
            next.set(event.presetId, {
              ...preset,
              isBuilding: false,
              buildProgress: null,
              freshlyBuiltAt: Date.now(),
            });
          }
          return next;
        });
        // Refresh bundle list after build
        void refreshBundles(event.presetId);
      }
      if (event.type === "bundleBuildProgress" && event.repoId === repoId) {
        const now = Date.now();
        const lastEntry = lastProgressUpdateRef.current.get(event.presetId);
        const shouldUpdate =
          !lastEntry ||
          event.phase !== lastEntry.phase ||
          event.currentFile !== lastEntry.currentFile ||
          event.filesDone !== lastEntry.filesDone ||
          event.filesTotal !== lastEntry.filesTotal ||
          now - lastEntry.ts >= PROGRESS_THROTTLE_MS;
        if (!shouldUpdate) {
          return;
        }
        const snapshot: {
          ts: number;
          phase: BundleBuildPhase;
          filesDone: number;
          filesTotal: number;
          currentFile?: string;
        } = {
          ts: now,
          phase: event.phase,
          filesDone: event.filesDone,
          filesTotal: event.filesTotal,
        };
        if (event.currentFile !== undefined) {
          snapshot.currentFile = event.currentFile;
        }
        lastProgressUpdateRef.current.set(event.presetId, snapshot);
        setPresets((prev) => {
          const next = new Map(prev);
          const preset = next.get(event.presetId);
          if (preset) {
            const progress: BundleBuildProgress = {
              phase: event.phase,
              filesDone: event.filesDone,
              filesTotal: event.filesTotal,
            };
            if (event.currentFile !== undefined) {
              progress.currentFile = event.currentFile;
            }
            if (event.currentBytesDone !== undefined) {
              progress.currentBytesDone = event.currentBytesDone;
            }
            if (event.currentBytesTotal !== undefined) {
              progress.currentBytesTotal = event.currentBytesTotal;
            }
            if (event.bytesDoneTotalBestEffort !== undefined) {
              progress.bytesDoneTotalBestEffort = event.bytesDoneTotalBestEffort;
            }
            next.set(event.presetId, {
              ...preset,
              isBuilding: true,
              buildProgress: progress,
              lastBuildError: null,
            });
          }
          return next;
        });
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
    const resetKey = `${repoId}|${buildPresetKey(repoPresets)}|${buildSelectionKey(
      savedSelections
    )}`;
    if (resetKeyRef.current === resetKey) {
      return;
    }
    resetKeyRef.current = resetKey;

    const next = new Map<string, BundlePresetState>();
    for (const preset of repoPresets) {
      const saved = savedSelections[preset.presetId];
      next.set(preset.presetId, createPresetState(preset, [], saved));
    }
    setPresets(next);
    setActivePresetId(repoPresets[0]?.presetId ?? DEFAULT_BUNDLE_PRESET.presetId);
    lastRefreshKeyRef.current = null;
  }, [repoId, repoPresets, savedSelections]);

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
              excludedSubdirs: preset.selection.excludedSubdirs,
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
    topLevelSubdirs,
    setSelection,
    buildBundle,
    setActivePreset: setActivePresetId,
    refreshBundles,
  };
}
