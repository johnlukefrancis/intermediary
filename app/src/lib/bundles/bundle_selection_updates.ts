// Path: app/src/lib/bundles/bundle_selection_updates.ts
// Description: Explicit bundle inclusion changes without enumerating or enabling sibling paths

import type { BundleSelection } from "../../shared/protocol.js";
import type { NormalizedGlobalExcludes } from "../../shared/global_exclude_rules.js";
import {
  isDirectoryEnabled, isDirectoryIncluded, isFileEnabled, isFileIncluded,
  isSelfOrDescendant, isTopLevelPath, sortedWith, withoutPath,
} from "./bundle_selection_visibility.js";

export function toggleDirectory(
  path: string, selection: BundleSelection, excludes: NormalizedGlobalExcludes
): BundleSelection {
  if (!isDirectoryEnabled(path, excludes)) return selection;
  const included = isDirectoryIncluded(path, selection, excludes);
  const outside = (value: string): boolean => !isSelfOrDescendant(value, path);
  if (included) {
    return {
      ...selection,
      topLevelDirs: withoutPath(path, selection.topLevelDirs),
      includedSubdirs: selection.includedSubdirs.filter(outside),
      includedFiles: selection.includedFiles.filter(outside),
      excludedSubdirs: isTopLevelPath(path) ? selection.excludedSubdirs.filter(outside)
        : sortedWith(path, selection.excludedSubdirs.filter(outside)),
      excludedFiles: selection.excludedFiles.filter(outside),
    };
  }
  return {
    ...selection,
    topLevelDirs: isTopLevelPath(path) ? sortedWith(path, selection.topLevelDirs) : selection.topLevelDirs,
    includedSubdirs: isTopLevelPath(path) ? selection.includedSubdirs : sortedWith(path, selection.includedSubdirs),
    excludedSubdirs: withoutPath(path, selection.excludedSubdirs),
  };
}

export function toggleFile(
  path: string, selection: BundleSelection, excludes: NormalizedGlobalExcludes
): BundleSelection {
  if (!isFileEnabled(path, selection, excludes)) return selection;
  const included = isFileIncluded(path, selection, excludes);
  return {
    ...selection,
    includedFiles: included ? withoutPath(path, selection.includedFiles) : sortedWith(path, selection.includedFiles),
    excludedFiles: included ? sortedWith(path, selection.excludedFiles) : withoutPath(path, selection.excludedFiles),
  };
}
