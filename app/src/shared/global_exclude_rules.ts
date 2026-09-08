// Path: app/src/shared/global_exclude_rules.ts
// Description: Normalize global excludes and match names and paths the way the bundle scanner does

import type { GlobalExcludes } from "./global_excludes.js";

/** Mirror of `im_bundle::global_excludes::NormalizedGlobalExcludes`: lowercased, empty entries dropped. */
export interface NormalizedGlobalExcludes {
  dirNames: string[];
  dirSuffixes: string[];
  fileNames: string[];
  fileSuffixes: string[];
  pathSegments: string[];
}

export function normalizeExtensionValue(value: string): string {
  const trimmed = value.trim().toLowerCase();
  if (trimmed.length === 0) return "";
  if (trimmed === "~") return "~";
  return trimmed.startsWith(".") ? trimmed : `.${trimmed}`;
}

export function normalizePatternValue(value: string): string {
  return value.trim().replace(/^\/+|\/+$/g, "").toLowerCase();
}

export function normalizeNameValue(value: string): string {
  return value.trim().toLowerCase();
}

function normalizedList(values: readonly string[], normalize: (value: string) => string): string[] {
  return values.map(normalize).filter((value) => value.length > 0);
}

export function normalizeGlobalExcludes(excludes: GlobalExcludes): NormalizedGlobalExcludes {
  return {
    dirNames: normalizedList(excludes.dirNames, normalizeNameValue),
    dirSuffixes: normalizedList(excludes.dirSuffixes, normalizeExtensionValue),
    fileNames: normalizedList(excludes.fileNames, normalizeNameValue),
    fileSuffixes: normalizedList(excludes.extensions, normalizeExtensionValue),
    pathSegments: normalizedList(excludes.patterns, normalizePatternValue),
  };
}

function matchesSuffix(loweredName: string, suffixes: readonly string[]): boolean {
  return suffixes.some((suffix) => loweredName.endsWith(suffix));
}

export function isGloballyExcludedDirName(name: string, excludes: NormalizedGlobalExcludes): boolean {
  const lowered = name.toLowerCase();
  return excludes.dirNames.includes(lowered) || matchesSuffix(lowered, excludes.dirSuffixes);
}

export function isGloballyExcludedFileName(name: string, excludes: NormalizedGlobalExcludes): boolean {
  const lowered = name.toLowerCase();
  return excludes.fileNames.includes(lowered) || matchesSuffix(lowered, excludes.fileSuffixes);
}

/** A path segment pattern matches the whole path, a leading, trailing, or interior segment run. */
export function isGloballyExcludedPath(archivePath: string, excludes: NormalizedGlobalExcludes): boolean {
  const lowered = archivePath.toLowerCase();
  return excludes.pathSegments.some(
    (pattern) =>
      lowered === pattern ||
      lowered.startsWith(`${pattern}/`) ||
      lowered.endsWith(`/${pattern}`) ||
      lowered.includes(`/${pattern}/`)
  );
}
