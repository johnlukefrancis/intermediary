// Path: app/src/lib/bundles/bundle_selection_visibility.ts
// Description: Shared path visibility helpers for bundle selection state

import type { BundleSelection } from "../../shared/protocol.js";
import {
  isGloballyExcludedDirName,
  isGloballyExcludedFileName,
  isGloballyExcludedPath,
  type NormalizedGlobalExcludes,
} from "../../shared/global_exclude_rules.js";

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

// Ancestor actions cover descendants; sending both would fail after moving the ancestor.
export function topmostPaths(paths: Iterable<string>): string[] {
  const all = [...new Set(paths)];
  return all.filter((path) => !all.some((other) => other !== path && isSelfOrDescendant(path, other)));
}

export function isDirectoryEnabled(path: string, excludes: NormalizedGlobalExcludes): boolean {
  return !isGloballyExcludedPath(path, excludes);
}

export function isDirectoryIncluded(
  path: string, selection: BundleSelection, excludes: NormalizedGlobalExcludes
): boolean {
  if (!isDirectoryEnabled(path, excludes)) return false;
  let included = false;
  const parts = path.split("/");
  for (let index = 0; index < parts.length; index += 1) {
    const directory = parts.slice(0, index + 1).join("/");
    if (selection.excludedSubdirs.includes(directory)) included = false;
    else if (selection.includedSubdirs.includes(directory) ||
      (index === 0 && selection.topLevelDirs.includes(directory))) included = true;
    else if (isGloballyExcludedDirName(parts[index] ?? "", excludes)) included = false;
  }
  return included;
}

export function directoryHasExclusions(path: string, selection: BundleSelection): boolean {
  const descendantPrefix = `${path}/`;
  return (
    selection.excludedSubdirs.some((subdir) => subdir.startsWith(descendantPrefix)) ||
    selection.excludedFiles.some((file) => file.startsWith(descendantPrefix))
  );
}

export function directoryHasInclusions(
  path: string, selection: BundleSelection, excludes: NormalizedGlobalExcludes
): boolean {
  const prefix = `${path}/`;
  return selection.includedSubdirs.some((dir) =>
    dir.startsWith(prefix) && isDirectoryIncluded(dir, selection, excludes)) ||
    selection.includedFiles.some((file) =>
      file.startsWith(prefix) && isFileIncluded(file, selection, excludes));
}

/** True when a global rule drops the file at build time whatever the selection says (`selection.rs`). */
export function isFileGloballyExcluded(path: string, excludes: NormalizedGlobalExcludes): boolean {
  return isGloballyExcludedFileName(baseName(path), excludes) || isGloballyExcludedPath(path, excludes);
}

export function isFileEnabled(
  path: string,
  _selection: BundleSelection,
  excludes: NormalizedGlobalExcludes
): boolean {
  return !isFileGloballyExcluded(path, excludes);
}

export function isFileIncluded(
  path: string,
  selection: BundleSelection,
  excludes: NormalizedGlobalExcludes
): boolean {
  if (!isFileEnabled(path, selection, excludes) || selection.excludedFiles.includes(path)) return false;
  if (selection.includedFiles.includes(path)) return true;
  const parent = parentPath(path);
  return parent === "" ? selection.includeRoot : isDirectoryIncluded(parent, selection, excludes);
}

export function sortedWith(path: string, values: readonly string[]): string[] {
  return [...new Set([...values, path])].sort();
}

export function withoutPath(path: string, values: readonly string[]): string[] {
  return values.filter((value) => value !== path);
}
