// Path: app/src/hooks/use_config_actions_remove.ts
// Description: Config actions that remove a repo or a whole group and every per-repo map entry they own

import { useCallback, type Dispatch, type SetStateAction } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { PersistedConfig, RepoConfig } from "../shared/config.js";

type SetConfig = Dispatch<SetStateAction<PersistedConfig>>;
type SaveConfig = (config: PersistedConfig) => void;

function retainDemoTargets(config: PersistedConfig, repos: readonly RepoConfig[]): Record<string, string> {
  const tabKeys = new Set(repos.map((repo) => repo.groupId ?? repo.repoId));
  return Object.fromEntries(Object.entries(config.demoTargets).filter(([key]) => tabKeys.has(key)));
}

function deleteRepoNote(repoId: string): void {
  void invoke("delete_note", { repoId }).catch((err: unknown) => {
    console.error(`[useConfigActions] Failed to delete note for repo "${repoId}"`, err);
  });
}

export function useRemoveRepo(
  setConfig: SetConfig,
  saveConfig: SaveConfig
): (repoId: string) => void {
  return useCallback(
    (repoId: string) => {
      setConfig((prev) => {
        const repoExists = prev.repos.some((repo) => repo.repoId === repoId);
        if (!repoExists) {
          return prev;
        }

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
        const { [repoId]: _removedLaunch, ...newLaunchTargets } =
          prev.launchTargets;

        const next: PersistedConfig = {
          ...prev,
          repos: newRepos,
          bundleSelections: newBundleSelections,
          uiState: newUiState,
          tabThemes: newTabThemes,
          starredFiles: newStarredFiles,
          launchTargets: newLaunchTargets,
          demoTargets: retainDemoTargets(prev, newRepos),
        };
        saveConfig(next);
        return next;
      });
      deleteRepoNote(repoId);
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
      let removedRepoIds: string[] = [];
      setConfig((prev) => {
        const reposToRemove = prev.repos.filter((r) => r.groupId === groupId);
        if (reposToRemove.length === 0) {
          return prev;
        }
        removedRepoIds = reposToRemove.map((repo) => repo.repoId);

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

        const updatedLaunchTargets = Object.fromEntries(
          Object.entries(prev.launchTargets).filter(
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
          launchTargets: updatedLaunchTargets,
          demoTargets: retainDemoTargets(prev, remainingRepos),
        };
        saveConfig(next);
        return next;
      });
      for (const repoId of removedRepoIds) {
        deleteRepoNote(repoId);
      }
    },
    [setConfig, saveConfig]
  );
}
