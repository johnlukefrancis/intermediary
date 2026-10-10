// Path: app/src/hooks/bundles/use_bundle_inclusion.ts
// Description: Bundle-selection inclusion callbacks (root/select-all/select-none/directory/file toggles) for the ZIPS explorer

import type React from "react";
import { useCallback } from "react";
import type { BundleSelection } from "../../shared/protocol.js";
import type { NormalizedGlobalExcludes } from "../../shared/global_exclude_rules.js";
import { toggleDirectory, toggleFile } from "../../lib/bundles/bundle_selection_updates.js";

interface UseBundleInclusionOptions {
  selection: BundleSelection;
  topLevelDirs: string[];
  globalExcludes: NormalizedGlobalExcludes;
  onSelectionChange: (selection: BundleSelection) => void;
}

export interface BundleInclusion {
  allSelected: boolean;
  noneSelected: boolean;
  handleIncludeRootChange: (event: React.ChangeEvent<HTMLInputElement>) => void;
  handleSelectAll: () => void;
  handleSelectNone: () => void;
  handleToggleDirectory: (path: string) => void;
  handleToggleFile: (path: string) => void;
}

export function useBundleInclusion({
  selection,
  topLevelDirs,
  globalExcludes,
  onSelectionChange,
}: UseBundleInclusionOptions): BundleInclusion {
  const allSelected =
    topLevelDirs.length > 0 && selection.topLevelDirs.length === topLevelDirs.length;
  const noneSelected = selection.topLevelDirs.length === 0 && selection.includedSubdirs.length === 0 &&
    !selection.includedFiles.some((path) => path.includes("/"));

  const handleIncludeRootChange = useCallback(
    (event: React.ChangeEvent<HTMLInputElement>) => {
      onSelectionChange({
        ...selection, includeRoot: event.target.checked,
        includedFiles: selection.includedFiles.filter((path) => path.includes("/")),
      });
    },
    [onSelectionChange, selection]
  );

  const handleSelectAll = useCallback(() => {
    onSelectionChange({ ...selection, topLevelDirs: [...topLevelDirs].sort() });
  }, [onSelectionChange, selection, topLevelDirs]);

  const handleSelectNone = useCallback(() => {
    onSelectionChange({
      ...selection,
      topLevelDirs: [],
      includedSubdirs: [],
      includedFiles: selection.includedFiles.filter((path) => !path.includes("/")),
      excludedSubdirs: [],
      excludedFiles: selection.excludedFiles.filter((value) => !value.includes("/")),
    });
  }, [onSelectionChange, selection]);

  const handleToggleDirectory = useCallback(
    (path: string) => { onSelectionChange(toggleDirectory(path, selection, globalExcludes)); },
    [globalExcludes, onSelectionChange, selection]
  );

  const handleToggleFile = useCallback(
    (path: string) => { onSelectionChange(toggleFile(path, selection, globalExcludes)); },
    [globalExcludes, onSelectionChange, selection]
  );

  return {
    allSelected,
    noneSelected,
    handleIncludeRootChange,
    handleSelectAll,
    handleSelectNone,
    handleToggleDirectory,
    handleToggleFile,
  };
}
