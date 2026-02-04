// Path: app/src/hooks/use_config_actions.ts
// Description: Core config action factory functions for repo and bundle management

import { useCallback, type Dispatch, type SetStateAction } from "react";
import {
  type PersistedConfig,
  type BundleSelection,
  type RepoConfig,
  type GlobalExcludes,
} from "../shared/config.js";
import { ACCENT_PALETTE, DEFAULT_ACCENT_HEX } from "../lib/theme/accent_utils.js";

type SetConfig = Dispatch<SetStateAction<PersistedConfig>>;
type SaveConfig = (config: PersistedConfig) => void;

function pickNextAccentHex(tabThemes: PersistedConfig["tabThemes"]): string {
  const used = new Set(
    Object.values(tabThemes).map((theme) => theme.accentHex.toLowerCase())
  );
  for (const color of ACCENT_PALETTE) {
    if (!used.has(color.toLowerCase())) {
      return color;
    }
  }
  if (ACCENT_PALETTE.length > 0) {
    const index = Object.keys(tabThemes).length % ACCENT_PALETTE.length;
    return ACCENT_PALETTE[index] ?? DEFAULT_ACCENT_HEX;
  }
  return DEFAULT_ACCENT_HEX;
}

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

export function useSetLastActiveGroupRepoId(
  setConfig: SetConfig,
  saveConfig: SaveConfig
): (groupId: string, repoId: string | null) => void {
  return useCallback(
    (groupId: string, repoId: string | null) => {
      setConfig((prev) => {
        const nextGroupMap = repoId
          ? {
              ...prev.uiState.lastActiveGroupRepoIds,
              [groupId]: repoId,
            }
          : Object.fromEntries(
              Object.entries(prev.uiState.lastActiveGroupRepoIds).filter(
                ([key]) => key !== groupId
              )
            );
        const next = {
          ...prev,
          uiState: {
            ...prev.uiState,
            lastActiveGroupRepoIds: nextGroupMap,
          },
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
        const themeKey = repo.groupId ?? repo.repoId;
        let nextTabThemes = prev.tabThemes;
        if (!(themeKey in prev.tabThemes)) {
          nextTabThemes = {
            ...prev.tabThemes,
            [themeKey]: {
              accentHex: pickNextAccentHex(prev.tabThemes),
            },
          };
        }
        const next: PersistedConfig = {
          ...prev,
          repos: [...prev.repos, repo],
          tabThemes: nextTabThemes,
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

export function useRenameRepoLabel(
  setConfig: SetConfig,
  saveConfig: SaveConfig
): (repoId: string, label: string) => void {
  return useCallback(
    (repoId: string, label: string) => {
      const trimmed = label.trim();
      if (trimmed.length === 0) return;
      setConfig((prev) => {
        const changed = prev.repos.some(
          (repo) => repo.repoId === repoId && repo.label !== trimmed
        );
        if (!changed) return prev;
        const updatedRepos = prev.repos.map((repo) =>
          repo.repoId === repoId ? { ...repo, label: trimmed } : repo
        );
        const next: PersistedConfig = { ...prev, repos: updatedRepos };
        saveConfig(next);
        return next;
      });
    },
    [setConfig, saveConfig]
  );
}

export function useRenameGroupLabel(
  setConfig: SetConfig,
  saveConfig: SaveConfig
): (groupId: string, label: string) => void {
  return useCallback(
    (groupId: string, label: string) => {
      const trimmed = label.trim();
      if (trimmed.length === 0) return;
      setConfig((prev) => {
        const changed = prev.repos.some(
          (repo) => repo.groupId === groupId && repo.groupLabel !== trimmed
        );
        if (!changed) return prev;
        const updatedRepos = prev.repos.map((repo) =>
          repo.groupId === groupId ? { ...repo, groupLabel: trimmed } : repo
        );
        const next: PersistedConfig = { ...prev, repos: updatedRepos };
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
        const removedRepo = prev.repos.find((r) => r.repoId === repoId);
        let nextGroupRepoIds = Object.fromEntries(
          Object.entries(prev.uiState.lastActiveGroupRepoIds).filter(
            ([, value]) => value !== repoId
          )
        );
        if (removedRepo?.groupId) {
          const fallback = prev.repos.find(
            (repo) => repo.groupId === removedRepo.groupId && repo.repoId !== repoId
          );
          if (fallback) {
            nextGroupRepoIds = {
              ...nextGroupRepoIds,
              [removedRepo.groupId]: fallback.repoId,
            };
          } else {
            const { [removedRepo.groupId]: _removed, ...rest } = nextGroupRepoIds;
            nextGroupRepoIds = rest;
          }
        }
        const newUiState = {
          ...prev.uiState,
          lastActiveTabId:
            prev.uiState.lastActiveTabId === repoId ? null : prev.uiState.lastActiveTabId,
          lastActiveGroupRepoIds: nextGroupRepoIds,
        };
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

export function useRemoveGroup(
  setConfig: SetConfig,
  saveConfig: SaveConfig
): (groupId: string) => void {
  return useCallback(
    (groupId: string) => {
      setConfig((prev) => {
        const reposToRemove = prev.repos.filter((r) => r.groupId === groupId);
        if (reposToRemove.length === 0) {
          return prev;
        }

        const repoIdsToRemove = new Set(reposToRemove.map((r) => r.repoId));
        const remainingRepos = prev.repos.filter((r) => r.groupId !== groupId);

        const updatedTabThemes = Object.fromEntries(
          Object.entries(prev.tabThemes).filter(
            ([key]) => key !== groupId && !repoIdsToRemove.has(key)
          )
        );

        const updatedBundleSelections = Object.fromEntries(
          Object.entries(prev.bundleSelections).filter(
            ([key]) => !repoIdsToRemove.has(key)
          )
        );

        const updatedStarredFiles = Object.fromEntries(
          Object.entries(prev.starredFiles).filter(
            ([key]) => !repoIdsToRemove.has(key)
          )
        );

        const updatedGroupRepoIds = Object.fromEntries(
          Object.entries(prev.uiState.lastActiveGroupRepoIds).filter(
            ([key, value]) => key !== groupId && !repoIdsToRemove.has(value)
          )
        );
        const newUiState = {
          ...prev.uiState,
          lastActiveTabId:
            prev.uiState.lastActiveTabId &&
            repoIdsToRemove.has(prev.uiState.lastActiveTabId)
              ? null
              : prev.uiState.lastActiveTabId,
          lastActiveGroupRepoIds: updatedGroupRepoIds,
        };

        const next: PersistedConfig = {
          ...prev,
          repos: remainingRepos,
          bundleSelections: updatedBundleSelections,
          uiState: newUiState,
          tabThemes: updatedTabThemes,
          starredFiles: updatedStarredFiles,
        };
        saveConfig(next);
        return next;
      });
    },
    [setConfig, saveConfig]
  );
}
