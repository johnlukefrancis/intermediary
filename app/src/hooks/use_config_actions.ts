// Path: app/src/hooks/use_config_actions.ts
// Description: Core config action factory functions for repo and bundle management

import { useCallback, type Dispatch, type SetStateAction } from "react";
import {
  type PersistedConfig,
  type BundleSelection,
  type RepoConfig,
  type GlobalExcludes,
} from "../shared/config.js";

type SetConfig = Dispatch<SetStateAction<PersistedConfig>>;
type SaveConfig = (config: PersistedConfig) => void;

export function useSetAutoStageGlobal(
  setConfig: SetConfig,
  saveConfig: SaveConfig
): (value: boolean) => void {
  return useCallback(
    (value: boolean) => {
      setConfig((prev) => {
        const next = { ...prev, autoStageGlobal: value };
        saveConfig(next);
        return next;
      });
    },
    [setConfig, saveConfig]
  );
}

export function useSetLastActiveTabId(
  setConfig: SetConfig,
  saveConfig: SaveConfig
): (repoId: string | null) => void {
  return useCallback(
    (repoId: string | null) => {
      setConfig((prev) => {
        const next = {
          ...prev,
          uiState: { ...prev.uiState, lastActiveTabId: repoId },
        };
        saveConfig(next);
        return next;
      });
    },
    [setConfig, saveConfig]
  );
}

export function useSetBundleSelection(
  setConfig: SetConfig,
  saveConfigNow: SaveConfig
): (repoId: string, presetId: string, selection: BundleSelection) => void {
  return useCallback(
    (repoId: string, presetId: string, selection: BundleSelection) => {
      setConfig((prev) => {
        const repoSelections = prev.bundleSelections[repoId] ?? {};
        const next: PersistedConfig = {
          ...prev,
          bundleSelections: {
            ...prev.bundleSelections,
            [repoId]: {
              ...repoSelections,
              [presetId]: selection,
            },
          },
        };
        saveConfigNow(next);
        return next;
      });
    },
    [setConfig, saveConfigNow]
  );
}

export function useSetGlobalExcludes(
  setConfig: SetConfig,
  saveConfig: SaveConfig
): (excludes: GlobalExcludes) => void {
  return useCallback(
    (excludes: GlobalExcludes) => {
      setConfig((prev) => {
        const next: PersistedConfig = {
          ...prev,
          globalExcludes: excludes,
        };
        saveConfig(next);
        return next;
      });
    },
    [setConfig, saveConfig]
  );
}

export function useAddRepo(
  setConfig: SetConfig,
  saveConfig: SaveConfig
): (repo: RepoConfig) => void {
  return useCallback(
    (repo: RepoConfig) => {
      setConfig((prev) => {
        const next: PersistedConfig = {
          ...prev,
          repos: [...prev.repos, repo],
        };
        saveConfig(next);
        return next;
      });
    },
    [setConfig, saveConfig]
  );
}

export function useUpdateRepo(
  setConfig: SetConfig,
  saveConfig: SaveConfig
): (repoId: string, updates: Partial<Omit<RepoConfig, "repoId">>) => void {
  return useCallback(
    (repoId: string, updates: Partial<Omit<RepoConfig, "repoId">>) => {
      setConfig((prev) => {
        const newRepos = prev.repos.map((r) =>
          r.repoId === repoId ? { ...r, ...updates } : r
        );
        const next: PersistedConfig = { ...prev, repos: newRepos };
        saveConfig(next);
        return next;
      });
    },
    [setConfig, saveConfig]
  );
}

export function useRemoveRepo(
  setConfig: SetConfig,
  saveConfig: SaveConfig
): (repoId: string) => void {
  return useCallback(
    (repoId: string) => {
      setConfig((prev) => {
        const newRepos = prev.repos.filter((r) => r.repoId !== repoId);
        const { [repoId]: _removed, ...newBundleSelections } =
          prev.bundleSelections;
        const newUiState =
          prev.uiState.lastActiveTabId === repoId
            ? { ...prev.uiState, lastActiveTabId: null }
            : prev.uiState;
        const { [repoId]: _removedTheme, ...newTabThemes } = prev.tabThemes;
        const { [repoId]: _removedStarred, ...newStarredFiles } =
          prev.starredFiles;

        const next: PersistedConfig = {
          ...prev,
          repos: newRepos,
          bundleSelections: newBundleSelections,
          uiState: newUiState,
          tabThemes: newTabThemes,
          starredFiles: newStarredFiles,
        };
        saveConfig(next);
        return next;
      });
    },
    [setConfig, saveConfig]
  );
}
