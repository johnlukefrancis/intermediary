// Path: app/src/hooks/use_starred_files.ts
// Description: Hook exposing starred file state and actions for a repo

import { useMemo, useCallback } from "react";
import { useConfig } from "./use_config.js";

interface UseStarredFilesResult {
  /** All starred docs paths for this repo */
  starredDocsPaths: readonly string[];
  /** All starred code paths for this repo */
  starredCodePaths: readonly string[];
  /** Check if a file is starred */
  isStarred: (kind: "docs" | "code", path: string) => boolean;
  /** Toggle a file's starred status */
  toggle: (kind: "docs" | "code", path: string) => void;
}

const EMPTY_ARRAY: readonly string[] = [];

export function useStarredFiles(repoId: string): UseStarredFilesResult {
  const { config, toggleStarredFile } = useConfig();

  const repoEntry = config.starredFiles[repoId];
  const starredDocsPaths = repoEntry?.docs ?? EMPTY_ARRAY;
  const starredCodePaths = repoEntry?.code ?? EMPTY_ARRAY;

  const isStarred = useCallback(
    (kind: "docs" | "code", path: string): boolean => {
      const list = kind === "docs" ? starredDocsPaths : starredCodePaths;
      return list.includes(path);
    },
    [starredDocsPaths, starredCodePaths]
  );

  const toggle = useCallback(
    (kind: "docs" | "code", path: string): void => {
      toggleStarredFile(repoId, kind, path);
    },
    [repoId, toggleStarredFile]
  );

  return useMemo(
    () => ({
      starredDocsPaths,
      starredCodePaths,
      isStarred,
      toggle,
    }),
    [starredDocsPaths, starredCodePaths, isStarred, toggle]
  );
}
