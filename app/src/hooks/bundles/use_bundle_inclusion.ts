// Path: app/src/hooks/bundles/use_bundle_inclusion.ts
// Description: Bundle-selection inclusion callbacks (root/select-all/select-none/directory/file toggles) for the ZIPS explorer

import type React from "react";
import { useCallback } from "react";
import type { BundleSelection } from "../../shared/protocol.js";
import {
  isSelfOrDescendant,
  sortedWith,
  withoutPath,
} from "../../lib/bundles/bundle_selection_visibility.js";

interface UseBundleInclusionOptions {
  selection: BundleSelection;
  topLevelDirs: string[];
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
  onSelectionChange,
}: UseBundleInclusionOptions): BundleInclusion {
  const allSelected =
    topLevelDirs.length > 0 && selection.topLevelDirs.length === topLevelDirs.length;
  const noneSelected = selection.topLevelDirs.length === 0;

  const handleIncludeRootChange = useCallback(
    (event: React.ChangeEvent<HTMLInputElement>) => {
      onSelectionChange({ ...selection, includeRoot: event.target.checked });
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
      excludedSubdirs: [],
      excludedFiles: selection.excludedFiles.filter((value) => !value.includes("/")),
    });
  }, [onSelectionChange, selection]);

  const handleToggleDirectory = useCallback(
    (path: string) => {
      if (!path.includes("/")) {
        const selected = new Set(selection.topLevelDirs);
        if (selected.has(path)) {
          selected.delete(path);
          onSelectionChange({
            ...selection,
            topLevelDirs: [...selected].sort(),
            includedSubdirs: selection.includedSubdirs.filter(
              (value) => !isSelfOrDescendant(value, path)
            ),
            excludedSubdirs: selection.excludedSubdirs.filter((value) => !isSelfOrDescendant(value, path)),
            excludedFiles: selection.excludedFiles.filter((value) => !isSelfOrDescendant(value, path)),
          });
        } else {
          selected.add(path);
          onSelectionChange({ ...selection, topLevelDirs: [...selected].sort() });
        }
        return;
      }

      const isExcluded = selection.excludedSubdirs.includes(path);
      const excludedSubdirs = isExcluded
        ? withoutPath(path, selection.excludedSubdirs)
        : sortedWith(path, selection.excludedSubdirs);
      const includedSubdirs = isExcluded
        ? sortedWith(path, selection.includedSubdirs)
        : selection.includedSubdirs.filter(
            (value) => !isSelfOrDescendant(value, path)
          );
      onSelectionChange({ ...selection, includedSubdirs, excludedSubdirs });
    },
    [onSelectionChange, selection]
  );

  const handleToggleFile = useCallback(
    (path: string) => {
      const excludedFiles = selection.excludedFiles.includes(path)
        ? withoutPath(path, selection.excludedFiles)
        : sortedWith(path, selection.excludedFiles);
      onSelectionChange({ ...selection, excludedFiles });
    },
    [onSelectionChange, selection]
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
