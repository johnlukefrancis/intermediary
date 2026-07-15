// Path: app/src/hooks/bundles/bundle_selection_defaults.ts
// Description: Bundle preset selection initialization and default-exclusion helpers

import { DEFAULT_BUNDLE_PRESET, type BundlePreset } from "../../shared/config.js";
import type { BundleSelection } from "../../shared/protocol.js";
import type { BundlePresetState } from "./bundle_state_types.js";

export const EMPTY_SAVED_SELECTIONS: Record<string, BundleSelection> = {};

export function buildPresetKey(presets: BundlePreset[]): string {
  return presets
    .map((preset) => {
      const dirs = preset.topLevelDirs.join(",");
      return `${preset.presetId}:${preset.presetName}:${preset.includeRoot}:${dirs}`;
    })
    .join("|");
}

export function buildSelectionKey(selections: Record<string, BundleSelection>): string {
  const entries = Object.keys(selections)
    .sort()
    .map((presetId) => {
      const selection = selections[presetId];
      if (!selection) {
        return `${presetId}:missing`;
      }
      const dirs = selection.topLevelDirs.join(",");
      const included = selection.includedSubdirs.join(",");
      const excluded = selection.excludedSubdirs.join(",");
      const excludedFiles = selection.excludedFiles.join(",");
      return `${presetId}:${selection.includeRoot}:${dirs}:${included}:${excluded}:${excludedFiles}`;
    });
  return entries.join("|");
}

export function normalizeTopLevelDirs(dirs: string[], available: string[] = []): string[] {
  const unique = Array.from(new Set(dirs.filter((dir) => dir.length > 0)));
  if (available.length === 0) {
    return unique.sort();
  }
  const allowed = new Set(available);
  return unique.filter((dir) => allowed.has(dir)).sort();
}

function subdirBaseName(path: string): string {
  const parts = path.split("/").filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

export function computeDefaultExcludedSubdirs(
  selectedDirs: string[],
  topLevelSubdirs: Record<string, string[]>,
  defaultExcluded: string[],
  includedSubdirs: readonly string[] = []
): string[] {
  if (defaultExcluded.length === 0) return [];
  const excludedSet = new Set(defaultExcluded);
  const includedSet = new Set(includedSubdirs);
  const result: string[] = [];
  for (const dir of selectedDirs) {
    const subs = topLevelSubdirs[dir];
    if (!subs) continue;
    for (const sub of subs) {
      const path = `${dir}/${sub}`;
      if (excludedSet.has(subdirBaseName(sub)) && !includedSet.has(path)) {
        result.push(path);
      }
    }
  }
  return result.sort();
}

export function mergeExcludedSubdirs(existing: string[], autoExcluded: string[]): string[] {
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

function createEmptyPresetState(
  preset: BundlePreset,
  selection: BundleSelection,
  isSelectionInitialized: boolean,
  isSelectionTopologyReady: boolean
): BundlePresetState {
  return {
    presetId: preset.presetId,
    presetName: preset.presetName,
    selection,
    isSelectionInitialized,
    isSelectionTopologyReady,
    isBuilding: false,
    isCancelling: false,
    activeBuildId: null,
    buildProgress: null,
    bundles: [],
    lastBuildError: null,
    freshlyBuiltAt: null,
  };
}

export function createPresetState(
  preset: BundlePreset,
  topLevelDirs: string[] = [],
  savedSelection?: BundleSelection,
  defaultExcluded: string[] = [],
  topLevelSubdirs: Record<string, string[]> = {},
  isTopologyReady = topLevelDirs.length > 0
): BundlePresetState {
  const activeDefaultExcluded = isTopologyReady ? defaultExcluded : [];
  const excludedSet = new Set(activeDefaultExcluded);

  if (savedSelection) {
    const normalizedDirs = isTopologyReady
      ? normalizeTopLevelDirs(savedSelection.topLevelDirs, topLevelDirs)
      : normalizeTopLevelDirs(savedSelection.topLevelDirs);
    const selectedDirs = normalizedDirs;
    const autoExcludedSubs = isTopologyReady
      ? computeDefaultExcludedSubdirs(
          selectedDirs,
          topLevelSubdirs,
          activeDefaultExcluded,
          savedSelection.includedSubdirs
        )
      : [];
    const mergedExcluded = mergeExcludedSubdirs(
      savedSelection.excludedSubdirs,
      autoExcludedSubs
    );
    return createEmptyPresetState(
      preset,
      {
        includeRoot: savedSelection.includeRoot,
        topLevelDirs: selectedDirs,
        includedSubdirs: [...savedSelection.includedSubdirs].sort(),
        excludedSubdirs: mergedExcluded.sort(),
        excludedFiles: [...savedSelection.excludedFiles].sort(),
      },
      true,
      isTopologyReady
    );
  }

  if (preset.topLevelDirs.length > 0) {
    const normalizedDirs = isTopologyReady
      ? normalizeTopLevelDirs(preset.topLevelDirs, topLevelDirs)
      : normalizeTopLevelDirs(preset.topLevelDirs);
    const selectedDirs = normalizedDirs.filter((d) => !excludedSet.has(d));
    return createEmptyPresetState(
      preset,
      {
        includeRoot: preset.includeRoot,
        topLevelDirs: selectedDirs,
        includedSubdirs: [],
        excludedSubdirs: isTopologyReady
          ? computeDefaultExcludedSubdirs(selectedDirs, topLevelSubdirs, activeDefaultExcluded)
          : [],
        excludedFiles: [],
      },
      isTopologyReady,
      isTopologyReady
    );
  }

  const selectedDirs = isTopologyReady
    ? [...topLevelDirs].filter((d) => !excludedSet.has(d)).sort()
    : [];
  return createEmptyPresetState(
    preset,
    {
      includeRoot: preset.includeRoot,
      topLevelDirs: selectedDirs,
      includedSubdirs: [],
      excludedSubdirs: isTopologyReady
        ? computeDefaultExcludedSubdirs(selectedDirs, topLevelSubdirs, activeDefaultExcluded)
        : [],
      excludedFiles: [],
    },
    isTopologyReady,
    isTopologyReady
  );
}

export function getRepoPresets(presets: BundlePreset[]): BundlePreset[] {
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
