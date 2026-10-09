// Path: app/src/hooks/use_app_navigation.ts
// Description: App view selection and remembered repository navigation

import { useCallback, useEffect } from "react";
import { useConfig } from "./use_config.js";

export function useAppNavigation(): {
  activeRepoId: string | null;
  overviewActive: boolean;
  selectRepo: (repoId: string) => void;
  openRepoStream: (repoId: string) => void;
  openOverview: () => void;
} {
  const { config, isLoaded, setLastActiveTabId, setLastActiveGroupRepoId, setMainView, setFilesMode } = useConfig();
  const remembered = config.uiState.lastActiveTabId;
  const activeRepoId = config.repos.some((repo) => repo.repoId === remembered)
    ? remembered : config.repos[0]?.repoId ?? null;

  useEffect(() => {
    if (isLoaded && remembered !== activeRepoId) setLastActiveTabId(activeRepoId);
  }, [activeRepoId, isLoaded, remembered, setLastActiveTabId]);

  const selectRepo = useCallback((repoId: string) => {
    setLastActiveTabId(repoId);
    const repo = config.repos.find((entry) => entry.repoId === repoId);
    if (repo?.groupId) setLastActiveGroupRepoId(repo.groupId, repoId);
    setMainView("repository");
  }, [config.repos, setLastActiveGroupRepoId, setLastActiveTabId, setMainView]);

  const openRepoStream = useCallback((repoId: string) => {
    setFilesMode("stream");
    selectRepo(repoId);
  }, [selectRepo, setFilesMode]);
  const openOverview = useCallback(() => { setMainView("overview"); }, [setMainView]);

  return { activeRepoId, overviewActive: config.uiState.mainView === "overview", selectRepo, openRepoStream, openOverview };
}
