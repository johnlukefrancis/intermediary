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
      const excluded = selection.excludedSubdirs.join(",");
      const excludedFiles = selection.excludedFiles.join(",");
      return `${presetId}:${selection.includeRoot}:${dirs}:${excluded}:${excludedFiles}`;
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
  defaultExcluded: string[]
): string[] {
  if (defaultExcluded.length === 0) return [];
  const excludedSet = new Set(defaultExcluded);
  const result: string[] = [];
  for (const dir of selectedDirs) {
    const subs = topLevelSubdirs[dir];
    if (!subs) continue;
    for (const sub of subs) {
      if (excludedSet.has(subdirBaseName(sub))) {
        result.push(`${dir}/${sub}`);
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
  isSelectionInitialized: boolean
): BundlePresetState {
  return {
    presetId: preset.presetId,
    presetName: preset.presetName,
    selection,
    isSelectionInitialized,
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
  topLevelSubdirs: Record<string, string[]> = {}
): BundlePresetState {
  const excludedSet = new Set(defaultExcluded);

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
    return createEmptyPresetState(
      preset,
      {
        includeRoot: savedSelection.includeRoot,
        topLevelDirs: normalizedDirs,
        excludedSubdirs: mergedExcluded.sort(),
        excludedFiles: [...savedSelection.excludedFiles].sort(),
      },
      true
    );
  }

  if (preset.topLevelDirs.length > 0) {
    const normalizedDirs = normalizeTopLevelDirs(preset.topLevelDirs, topLevelDirs);
    const selectedDirs = normalizedDirs.filter((d) => !excludedSet.has(d));
    return createEmptyPresetState(
      preset,
      {
        includeRoot: preset.includeRoot,
        topLevelDirs: selectedDirs,
        excludedSubdirs: computeDefaultExcludedSubdirs(
          selectedDirs, topLevelSubdirs, defaultExcluded
        ),
        excludedFiles: [],
      },
      true
    );
  }

  const selectedDirs = topLevelDirs.length > 0
    ? [...topLevelDirs].filter((d) => !excludedSet.has(d)).sort()
    : [];
  return createEmptyPresetState(
    preset,
    {
      includeRoot: preset.includeRoot,
      topLevelDirs: selectedDirs,
      excludedSubdirs: computeDefaultExcludedSubdirs(
        selectedDirs, topLevelSubdirs, defaultExcluded
      ),
      excludedFiles: [],
    },
    topLevelDirs.length > 0
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
