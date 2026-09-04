// Path: app/src/lib/bundles/bundle_selection_visibility.ts
// Description: Shared path visibility helpers for bundle selection state

import type { BundleSelection } from "../../shared/protocol.js";

export function baseName(path: string): string {
  const parts = path.split("/").filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

export function parentPath(path: string): string {
  const parts = path.split("/").filter(Boolean);
  if (parts.length <= 1) return "";
  return parts.slice(0, -1).join("/");
}

export function isTopLevelPath(path: string): boolean {
  return !path.includes("/");
}

export function isSelfOrDescendant(path: string, ancestor: string): boolean {
  return path === ancestor || path.startsWith(`${ancestor}/`);
}

/**
 * The paths with no selected ancestor. A worktree action on a folder already covers everything
 * beneath it, so a selection of `app` plus `app/a.txt` is sent as `app` alone; sending both would
 * make the second entry fail after the first had moved it.
 */
export function topmostPaths(paths: Iterable<string>): string[] {
  const all = [...new Set(paths)];
  return all.filter((path) => !all.some((other) => other !== path && isSelfOrDescendant(path, other)));
}

function hasExcludedAncestor(path: string, excludedSubdirs: readonly string[]): boolean {
  const parts = path.split("/").filter(Boolean);
  for (let index = 1; index < parts.length; index += 1) {
    const ancestor = parts.slice(0, index).join("/");
    if (excludedSubdirs.includes(ancestor)) {
      return true;
    }
  }
  return false;
}

export function isDirectoryEnabled(path: string, selection: BundleSelection): boolean {
  if (isTopLevelPath(path)) return true;
  const topLevel = path.split("/")[0] ?? path;
  return (
    selection.topLevelDirs.includes(topLevel) &&
    !hasExcludedAncestor(path, selection.excludedSubdirs)
  );
}

export function isDirectoryIncluded(path: string, selection: BundleSelection): boolean {
  if (!isDirectoryEnabled(path, selection)) return false;
  if (isTopLevelPath(path)) return selection.topLevelDirs.includes(path);
  return !selection.excludedSubdirs.includes(path);
}

export function directoryHasExclusions(path: string, selection: BundleSelection): boolean {
  const descendantPrefix = `${path}/`;
  return (
    selection.excludedSubdirs.some((subdir) => subdir.startsWith(descendantPrefix)) ||
    selection.excludedFiles.some((file) => file.startsWith(descendantPrefix))
  );
}

export function isFileEnabled(path: string, selection: BundleSelection): boolean {
  const parent = parentPath(path);
  if (parent === "") return selection.includeRoot;
  return isDirectoryIncluded(parent, selection);
}

export function isFileIncluded(path: string, selection: BundleSelection): boolean {
  return isFileEnabled(path, selection) && !selection.excludedFiles.includes(path);
}

export function sortedWith(path: string, values: readonly string[]): string[] {
  return [...values, path].sort();
}

export function withoutPath(path: string, values: readonly string[]): string[] {
  return values.filter((value) => value !== path);
}
