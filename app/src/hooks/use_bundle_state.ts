// Path: app/src/hooks/use_bundle_state.ts
// Description: Per-repo bundle state management with event subscription

import { useState, useEffect, useCallback, useMemo, useRef } from "react";
import { useAgent } from "./use_agent.js";
import { useConfig } from "./use_config.js";
import {
  buildPresetKey,
  buildSelectionKey,
  computeDefaultExcludedSubdirs,
  createPresetState,
  EMPTY_SAVED_SELECTIONS,
  getRepoPresets,
  mergeExcludedSubdirs,
} from "./bundles/bundle_selection_defaults.js";
import type {
  BundlePresetState,
  BundleProgressThrottleEntry,
  BundleState,
} from "./bundles/bundle_state_types.js";
import { useBundleBuildActions } from "./bundles/use_bundle_build_actions.js";
import { useBundleEvents } from "./bundles/use_bundle_events.js";
import { useBundleRefresh } from "./bundles/use_bundle_refresh.js";
import type { BundleSelection } from "../shared/protocol.js";
import { DEFAULT_BUNDLE_PRESET } from "../shared/config.js";
export type { BundlePresetState, BundleState } from "./bundles/bundle_state_types.js";

export function useBundleState(
  repoId: string,
  topLevelDirs: string[],
  topLevelSubdirs: Record<string, string[]>,
  defaultExcluded: string[] = [],
  isTopologyReady = topLevelDirs.length > 0
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
        preset, topLevelDirs, saved, defaultExcluded, topLevelSubdirs, isTopologyReady
      ));
    }
    return initial;
  });
  const [activePresetId, setActivePresetId] = useState(
    repoPresets[0]?.presetId ?? DEFAULT_BUNDLE_PRESET.presetId
  );
  const lastRefreshKeyRef = useRef<string | null>(null);
  const resetKeyRef = useRef<string | null>(null);
  const lastProgressUpdateRef = useRef<Map<string, BundleProgressThrottleEntry>>(new Map());
  const refreshRetryTimersRef = useRef<Map<string, ReturnType<typeof setTimeout>>>(new Map());
  const refreshRetryAttemptsRef = useRef<Map<string, number>>(new Map());
  const refreshInFlightRef = useRef<Set<string>>(new Set());
  const refreshEpochRef = useRef(0);

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
          isSelectionTopologyReady: isTopologyReady,
          lastBuildError: null,
        });
      }
      return next;
    });
    // Persist to config
    persistSelection(repoId, presetId, selection);
  }, [isTopologyReady, repoId, persistSelection]);

  const { clearAllRefreshRetries, refreshBundles } = useBundleRefresh({
    client,
    connectionStatus: connectionState.status,
    helloStatus: helloState.status,
    repoId,
    resyncClientHello,
    setPresets,
    refreshRetryTimersRef,
    refreshRetryAttemptsRef,
    refreshInFlightRef,
    refreshEpochRef,
  });
  const { buildBundle, cancelBundleBuild } = useBundleBuildActions({
    client,
    connectionStatus: connectionState.status,
    helloStatus: helloState.status,
    globalExcludes: persistedConfig.globalExcludes,
    presets,
    repoId,
    resyncClientHello,
    refreshBundles,
    setPresets,
  });
  const handleEvent = useBundleEvents({
    repoId,
    refreshBundles,
    setPresets,
    lastProgressUpdateRef,
  });

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
      next.set(preset.presetId, createPresetState(
        preset, topLevelDirs, saved, defaultExcluded, topLevelSubdirs, isTopologyReady
      ));
    }
    setPresets(next);
    setActivePresetId(repoPresets[0]?.presetId ?? DEFAULT_BUNDLE_PRESET.presetId);
    lastRefreshKeyRef.current = null;
  }, [
    repoId,
    repoPresets,
    savedSelections,
    defaultExcluded,
    topLevelDirs,
    topLevelSubdirs,
    isTopologyReady,
  ]);

  useEffect(() => {
    if (!isTopologyReady) {
      return;
    }
    const excludedSet = new Set(defaultExcluded);
    setPresets((prev) => {
      let changed = false;
      const next = new Map(prev);
      for (const [presetId, preset] of next) {
        if (!preset.isSelectionTopologyReady) {
          const presetConfig = repoPresets.find((candidate) => candidate.presetId === presetId);
          if (!presetConfig) continue;
          const saved = savedSelections[presetId];
          const initialized = createPresetState(
            presetConfig,
            topLevelDirs,
            saved,
            defaultExcluded,
            topLevelSubdirs,
            true
          );
          next.set(presetId, {
            ...preset,
            selection: initialized.selection,
            isSelectionInitialized: initialized.isSelectionInitialized,
            isSelectionTopologyReady: true,
          });
          changed = true;
          continue;
        }

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
              excludedFiles: preset.selection.excludedFiles,
            },
            isSelectionInitialized: true,
            isSelectionTopologyReady: true,
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
  }, [
    topLevelDirs,
    defaultExcluded,
    topLevelSubdirs,
    isTopologyReady,
    repoPresets,
    savedSelections,
  ]);

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
    cancelBundleBuild,
    setActivePreset: setActivePresetId,
    refreshBundles,
  };
}
