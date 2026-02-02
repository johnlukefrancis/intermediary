// Path: app/src/components/options/excludes/excludes_updates.ts
// Description: Pure update helpers for global excludes toggles

import type { GlobalExcludes } from "../../../shared/global_excludes.js";
import {
  normalizeExtension,
  normalizeName,
  normalizePattern,
} from "./excludes_normalizers.js";

export interface NormalizedValues {
  extensions: string[];
  patterns: string[];
  dirNames: string[];
  dirSuffixes: string[];
  fileNames: string[];
}

export function updateExtensions(
  values: NormalizedValues,
  value: string,
  enabled: boolean
): GlobalExcludes {
  const normalized = normalizeExtension(value);
  const extensionSet = new Set(values.extensions);
  if (enabled) {
    extensionSet.add(normalized);
  } else {
    extensionSet.delete(normalized);
  }
  return {
    dirNames: values.dirNames,
    dirSuffixes: values.dirSuffixes,
    fileNames: values.fileNames,
    extensions: Array.from(extensionSet),
    patterns: values.patterns,
  };
}

export function updatePatterns(
  values: NormalizedValues,
  value: string,
  enabled: boolean
): GlobalExcludes {
  const normalized = normalizePattern(value);
  const patternSet = new Set(values.patterns);
  if (enabled) {
    patternSet.add(normalized);
  } else {
    patternSet.delete(normalized);
  }
  return {
    dirNames: values.dirNames,
    dirSuffixes: values.dirSuffixes,
    fileNames: values.fileNames,
    extensions: values.extensions,
    patterns: Array.from(patternSet),
  };
}

export function updateDirNames(
  values: NormalizedValues,
  value: string,
  enabled: boolean
): GlobalExcludes {
  const normalized = normalizePattern(value);
  const dirSet = new Set(values.dirNames);
  if (enabled) {
    dirSet.add(normalized);
  } else {
    dirSet.delete(normalized);
  }
  return {
    dirNames: Array.from(dirSet),
    dirSuffixes: values.dirSuffixes,
    fileNames: values.fileNames,
    extensions: values.extensions,
    patterns: values.patterns,
  };
}

export function updateDirSuffixes(
  values: NormalizedValues,
  value: string,
  enabled: boolean
): GlobalExcludes {
  const normalized = normalizeExtension(value);
  const suffixSet = new Set(values.dirSuffixes);
  if (enabled) {
    suffixSet.add(normalized);
  } else {
    suffixSet.delete(normalized);
  }
  return {
    dirNames: values.dirNames,
    dirSuffixes: Array.from(suffixSet),
    fileNames: values.fileNames,
    extensions: values.extensions,
    patterns: values.patterns,
  };
}

export function updateFileNames(
  values: NormalizedValues,
  value: string,
  enabled: boolean
): GlobalExcludes {
  const normalized = normalizeName(value);
  const fileSet = new Set(values.fileNames);
  if (enabled) {
    fileSet.add(normalized);
  } else {
    fileSet.delete(normalized);
  }
  return {
    dirNames: values.dirNames,
    dirSuffixes: values.dirSuffixes,
    fileNames: Array.from(fileSet),
    extensions: values.extensions,
    patterns: values.patterns,
  };
}
