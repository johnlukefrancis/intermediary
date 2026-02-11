// Path: app/src/hooks/use_bundle_state.ts
// Description: Per-repo bundle state management with event subscription

import { useState, useEffect, useCallback, useMemo, useRef } from "react";
import { useAgent } from "./use_agent.js";
import { useConfig } from "./use_config.js";
import { sendBuildBundle, sendListBundles } from "../lib/agent/messages.js";
import {
  computeTransientRetryDelayMs,
  isTransientWslTransportError,
} from "../lib/agent/transient_wsl_error.js";
import { isStagingNotConfiguredError } from "../lib/agent/error_codes.js";
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

function computeDefaultExcludedSubdirs(
  selectedDirs: string[],
  topLevelSubdirs: Record<string, string[]>,
  defaultExcluded: string[]
): string[] {
  if (defaultExcluded.length === 0) return [];
  const excludedSet = new Set(defaultExcluded);
  const result: string[] = [];
  for (const dir of selectedDirs) {
    const subs = topLevelSubdirs[dir];
    if (!subs) continue;
    for (const sub of subs) {
      if (excludedSet.has(sub)) {
        result.push(`${dir}/${sub}`);
      }
    }
  }
  return result.sort();
}

function mergeExcludedSubdirs(existing: string[], autoExcluded: string[]): string[] {
  if (autoExcluded.length === 0) {
    return existing;
  }
  const merged = new Set(existing);
  for (const path of autoExcluded) {
    merged.add(path);
  }
  if (merged.size === existing.length) {
    return existing;
  }
  return Array.from(merged).sort();
}

function createPresetState(
  preset: BundlePreset,
  topLevelDirs: string[] = [],
  savedSelection?: BundleSelection,
  defaultExcluded: string[] = [],
  topLevelSubdirs: Record<string, string[]> = {}
): BundlePresetState {
  const excludedSet = new Set(defaultExcluded);

  // If we have a saved selection, use it — but auto-add default-excluded subdirs
  if (savedSelection) {
    const normalizedDirs = normalizeTopLevelDirs(savedSelection.topLevelDirs, topLevelDirs);
    const autoExcludedSubs = computeDefaultExcludedSubdirs(
      normalizedDirs, topLevelSubdirs, defaultExcluded
    );
    const existingExcluded = new Set(savedSelection.excludedSubdirs);
    const mergedExcluded = [...savedSelection.excludedSubdirs];
    for (const sub of autoExcludedSubs) {
      if (!existingExcluded.has(sub)) {
        mergedExcluded.push(sub);
      }
    }
    return {
      presetId: preset.presetId,
      presetName: preset.presetName,
      selection: {
        includeRoot: savedSelection.includeRoot,
        topLevelDirs: normalizedDirs,
        excludedSubdirs: mergedExcluded.sort(),
      },
      isSelectionInitialized: true,
      isBuilding: false,
      buildProgress: null,
      bundles: [],
      lastBuildError: null,
      freshlyBuiltAt: null,
    };
  }

  // If preset has explicit dirs, use those — filter default-excluded from selection
  if (preset.topLevelDirs.length > 0) {
    const normalizedDirs = normalizeTopLevelDirs(preset.topLevelDirs, topLevelDirs);
    const selectedDirs = normalizedDirs.filter((d) => !excludedSet.has(d));
    const autoExcludedSubs = computeDefaultExcludedSubdirs(
      selectedDirs, topLevelSubdirs, defaultExcluded
    );
    return {
      presetId: preset.presetId,
      presetName: preset.presetName,
      selection: {
        includeRoot: preset.includeRoot,
        topLevelDirs: selectedDirs,
        excludedSubdirs: autoExcludedSubs,
      },
      isSelectionInitialized: true,
      isBuilding: false,
      buildProgress: null,
      bundles: [],
      lastBuildError: null,
      freshlyBuiltAt: null,
    };
  }

  // Default to all available dirs — filter default-excluded from selection
  const selectedDirs = topLevelDirs.length > 0
    ? [...topLevelDirs].filter((d) => !excludedSet.has(d)).sort()
    : [];
  const autoExcludedSubs = computeDefaultExcludedSubdirs(
    selectedDirs, topLevelSubdirs, defaultExcluded
  );
  return {
    presetId: preset.presetId,
    presetName: preset.presetName,
    selection: {
      includeRoot: preset.includeRoot,
      topLevelDirs: selectedDirs,
      excludedSubdirs: autoExcludedSubs,
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
  topLevelSubdirs: Record<string, string[]>,
  defaultExcluded: string[] = []
): BundleState {
  const {
    subscribe,
    client,
    connectionState,
    helloState,
    rehydrateEpoch,
    config,
    resyncClientHello,
  } = useAgent();
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
      initial.set(preset.presetId, createPresetState(
        preset, topLevelDirs, saved, defaultExcluded, topLevelSubdirs
      ));
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
  const refreshRetryTimersRef = useRef<Map<string, ReturnType<typeof setTimeout>>>(new Map());
  const refreshRetryAttemptsRef = useRef<Map<string, number>>(new Map());
  const refreshInFlightRef = useRef<Set<string>>(new Set());
  const refreshEpochRef = useRef(0);
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

  const clearRefreshRetry = useCallback((presetId: string) => {
    const timer = refreshRetryTimersRef.current.get(presetId);
    if (timer) {
      clearTimeout(timer);
      refreshRetryTimersRef.current.delete(presetId);
    }
    refreshRetryAttemptsRef.current.delete(presetId);
  }, []);

  const clearAllRefreshRetries = useCallback(() => {
    for (const timer of refreshRetryTimersRef.current.values()) {
      clearTimeout(timer);
    }
    refreshRetryTimersRef.current.clear();
    refreshRetryAttemptsRef.current.clear();
    refreshInFlightRef.current.clear();
  }, []);

  // Refresh bundle list for a preset
  const refreshBundles = useCallback(
    async (presetId: string) => {
      if (
        !client ||
        connectionState.status !== "connected" ||
        helloState.status !== "ok"
      ) {
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
      connectionState.status,
      helloState.status,
      repoId,
      resyncClientHello,
    ]
  );

  // Build bundle for a preset
  const buildBundle = useCallback(
    async (presetId: string) => {
      if (!client || connectionState.status !== "connected") return;

      const preset = presets.get(presetId);
      if (!preset || preset.isBuilding) return;
      if (helloState.status !== "ok") {
        setPresets((prev) => {
          const next = new Map(prev);
          const p = next.get(presetId);
          if (p) {
            next.set(presetId, {
              ...p,
              isBuilding: false,
              buildProgress: null,
              lastBuildError: "Agent session initializing; retry in a moment.",
            });
          }
          return next;
        });
        return;
      }

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
        try {
          await sendBuildBundle(client, repoId, presetId, preset.selection, globalExcludes);
        } catch (err) {
          if (!isStagingNotConfiguredError(err)) {
            throw err;
          }
          const resynced = await resyncClientHello();
          if (!resynced) {
            throw err;
          }
          await sendBuildBundle(client, repoId, presetId, preset.selection, globalExcludes);
        }
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
    [
      client,
      connectionState.status,
      helloState.status,
      persistedConfig.globalExcludes,
      presets,
      refreshBundles,
      repoId,
      resyncClientHello,
    ]
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
              lastBuildError: null,
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

  useEffect(() => {
    refreshEpochRef.current += 1;
    clearAllRefreshRetries();
  }, [
    repoId,
    connectionState.status,
    helloState.lastHelloAt,
    rehydrateEpoch,
    clearAllRefreshRetries,
  ]);

  useEffect(() => {
    return () => {
      clearAllRefreshRetries();
    };
  }, [clearAllRefreshRetries]);

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
      next.set(preset.presetId, createPresetState(preset, [], saved, defaultExcluded));
    }
    setPresets(next);
    setActivePresetId(repoPresets[0]?.presetId ?? DEFAULT_BUNDLE_PRESET.presetId);
    lastRefreshKeyRef.current = null;
  }, [repoId, repoPresets, savedSelections, defaultExcluded]);

  useEffect(() => {
    if (topLevelDirs.length === 0) {
      return;
    }
    const excludedSet = new Set(defaultExcluded);
    setPresets((prev) => {
      let changed = false;
      const next = new Map(prev);
      for (const preset of next.values()) {
        if (!preset.isSelectionInitialized) {
          const selectedDirs = [...topLevelDirs].filter((d) => !excludedSet.has(d)).sort();
          const autoExcludedSubs = computeDefaultExcludedSubdirs(
            selectedDirs, topLevelSubdirs, defaultExcluded
          );
          next.set(preset.presetId, {
            ...preset,
            selection: {
              includeRoot: preset.selection.includeRoot,
              topLevelDirs: selectedDirs,
              excludedSubdirs: autoExcludedSubs,
            },
            isSelectionInitialized: true,
          });
          changed = true;
          continue;
        }

        const autoExcludedSubs = computeDefaultExcludedSubdirs(
          preset.selection.topLevelDirs, topLevelSubdirs, defaultExcluded
        );
        const mergedExcluded = mergeExcludedSubdirs(
          preset.selection.excludedSubdirs,
          autoExcludedSubs
        );
        if (mergedExcluded === preset.selection.excludedSubdirs) {
          continue;
        }
        next.set(preset.presetId, {
          ...preset,
          selection: {
            ...preset.selection,
            excludedSubdirs: mergedExcluded,
          },
        });
        changed = true;
      }
      return changed ? next : prev;
    });
  }, [topLevelDirs, defaultExcluded, topLevelSubdirs]);

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
    const refreshKey = `${repoId}:${activePresetId}:${helloState.lastHelloAt}:${rehydrateEpoch}`;
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
    rehydrateEpoch,
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
