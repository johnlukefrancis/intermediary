// Path: app/src/components/options/excludes/use_excludes_state.ts
// Description: State and handlers for the excludes section UI

import type React from "react";
import { useCallback, useMemo, useState } from "react";
import type { GlobalExcludes } from "../../../shared/global_excludes.js";
import {
  GLOBAL_EXCLUDE_DIR_OPTIONS,
  GLOBAL_EXCLUDE_DIR_SUFFIX_OPTIONS,
  GLOBAL_EXCLUDE_EXTENSION_OPTIONS,
  GLOBAL_EXCLUDE_FILE_OPTIONS,
  GLOBAL_EXCLUDE_SUFFIX_OPTIONS,
  GLOBAL_EXCLUDE_PATTERN_OPTIONS,
} from "../../../shared/global_excludes.js";
import {
  normalizeExtension,
  normalizeName,
  normalizePattern,
} from "./excludes_normalizers.js";
import {
  buildRecommendedExcludes,
  isRecommendedEnabled,
} from "./excludes_recommendations.js";
import {
  updateDirNames,
  updateDirSuffixes,
  updateExtensions,
  updateFileNames,
  updatePatterns,
  type NormalizedValues,
} from "./excludes_updates.js";

const OPTIONS_SECTIONS = [
  {
    key: "directories",
    title: "Directories",
    options: GLOBAL_EXCLUDE_DIR_OPTIONS,
    select: (values: NormalizedValues) => values.dirNames,
    onToggle: "handleDirToggle",
  },
  {
    key: "dirSuffixes",
    title: "Directory Suffixes",
    options: GLOBAL_EXCLUDE_DIR_SUFFIX_OPTIONS,
    select: (values: NormalizedValues) => values.dirSuffixes,
    onToggle: "handleDirSuffixToggle",
  },
  {
    key: "fileNames",
    title: "File Names",
    options: GLOBAL_EXCLUDE_FILE_OPTIONS,
    select: (values: NormalizedValues) => values.fileNames,
    onToggle: "handleFileNameToggle",
  },
  {
    key: "fileSuffixes",
    title: "File Suffixes",
    options: GLOBAL_EXCLUDE_SUFFIX_OPTIONS,
    select: (values: NormalizedValues) => values.extensions,
    onToggle: "handleExtensionToggle",
  },
  {
    key: "extensions",
    title: "Extensions",
    options: GLOBAL_EXCLUDE_EXTENSION_OPTIONS,
    select: (values: NormalizedValues) => values.extensions,
    onToggle: "handleExtensionToggle",
  },
  {
    key: "patterns",
    title: "Path Segments",
    options: GLOBAL_EXCLUDE_PATTERN_OPTIONS,
    select: (values: NormalizedValues) => values.patterns,
    onToggle: "handlePatternToggle",
  },
] as const;

interface UseExcludesStateArgs {
  excludes: GlobalExcludes;
  setExcludes: (excludes: GlobalExcludes) => void;
}

export function useExcludesState({
  excludes,
  setExcludes,
}: UseExcludesStateArgs) {
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [advancedSectionsOpen, setAdvancedSectionsOpen] = useState({
    directories: false,
    dirSuffixes: false,
    fileNames: false,
    fileSuffixes: false,
    extensions: false,
    patterns: false,
  });

  const toggleAdvanced = useCallback(() => {
    setAdvancedOpen((prev) => !prev);
  }, []);

  const toggleAdvancedSection = useCallback(
    (key: keyof typeof advancedSectionsOpen) => {
      setAdvancedSectionsOpen((prev) => ({ ...prev, [key]: !prev[key] }));
    },
    []
  );

  const normalizedExtensions = useMemo(
    () => excludes.extensions.map(normalizeExtension).filter((value) => value.length > 0),
    [excludes.extensions]
  );
  const normalizedPatterns = useMemo(
    () => excludes.patterns.map(normalizePattern).filter((value) => value.length > 0),
    [excludes.patterns]
  );
  const normalizedDirNames = useMemo(
    () => excludes.dirNames.map(normalizePattern).filter((value) => value.length > 0),
    [excludes.dirNames]
  );
  const normalizedDirSuffixes = useMemo(
    () => excludes.dirSuffixes.map(normalizeExtension).filter((value) => value.length > 0),
    [excludes.dirSuffixes]
  );
  const normalizedFileNames = useMemo(
    () => excludes.fileNames.map(normalizeName).filter((value) => value.length > 0),
    [excludes.fileNames]
  );

  const normalizedValues: NormalizedValues = useMemo(
    () => ({
      extensions: normalizedExtensions,
      patterns: normalizedPatterns,
      dirNames: normalizedDirNames,
      dirSuffixes: normalizedDirSuffixes,
      fileNames: normalizedFileNames,
    }),
    [
      normalizedExtensions,
      normalizedPatterns,
      normalizedDirNames,
      normalizedDirSuffixes,
      normalizedFileNames,
    ]
  );

  const recommendedEnabled = useMemo(() => {
    return isRecommendedEnabled({
      extensions: new Set(normalizedExtensions),
      patterns: new Set(normalizedPatterns),
      dirNames: new Set(normalizedDirNames),
      dirSuffixes: new Set(normalizedDirSuffixes),
      fileNames: new Set(normalizedFileNames),
    });
  }, [
    normalizedDirNames,
    normalizedDirSuffixes,
    normalizedExtensions,
    normalizedFileNames,
    normalizedPatterns,
  ]);

  const handleRecommendedToggle = useCallback(
    (event: React.ChangeEvent<HTMLInputElement>) => {
      const enabled = event.target.checked;
      setExcludes(buildRecommendedExcludes(enabled));
    },
    [setExcludes]
  );

  const handleExtensionToggle = useCallback(
    (value: string, enabled: boolean) => {
      setExcludes(
        updateExtensions(normalizedValues, value, enabled)
      );
    },
    [
      normalizedValues,
      setExcludes,
    ]
  );

  const handlePatternToggle = useCallback(
    (value: string, enabled: boolean) => {
      setExcludes(
        updatePatterns(normalizedValues, value, enabled)
      );
    },
    [
      normalizedValues,
      setExcludes,
    ]
  );

  const handleDirToggle = useCallback(
    (value: string, enabled: boolean) => {
      setExcludes(
        updateDirNames(normalizedValues, value, enabled)
      );
    },
    [
      normalizedValues,
      setExcludes,
    ]
  );

  const handleDirSuffixToggle = useCallback(
    (value: string, enabled: boolean) => {
      setExcludes(
        updateDirSuffixes(normalizedValues, value, enabled)
      );
    },
    [
      normalizedValues,
      setExcludes,
    ]
  );

  const handleFileNameToggle = useCallback(
    (value: string, enabled: boolean) => {
      setExcludes(
        updateFileNames(normalizedValues, value, enabled)
      );
    },
    [
      normalizedValues,
      setExcludes,
    ]
  );

  const toggleHandlers = useMemo(
    () => ({
      handleDirToggle,
      handleDirSuffixToggle,
      handleFileNameToggle,
      handleExtensionToggle,
      handlePatternToggle,
    }),
    [
      handleDirToggle,
      handleDirSuffixToggle,
      handleFileNameToggle,
      handleExtensionToggle,
      handlePatternToggle,
    ]
  );

  return {
    advancedOpen,
    advancedSectionsOpen,
    toggleAdvanced,
    toggleAdvancedSection,
    recommendedEnabled,
    handleRecommendedToggle,
    normalizedValues,
    toggleHandlers,
    optionsSections: OPTIONS_SECTIONS,
  };
}
