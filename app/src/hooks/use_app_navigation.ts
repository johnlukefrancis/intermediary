// Path: app/src/hooks/use_app_navigation.ts
// Description: App view selection, remembered repository navigation and the handset landing intent for a repo tab

import { useCallback, useEffect, useState } from "react";
import type { DeckLanding } from "./use_deck_section.js";
import { useConfig } from "./use_config.js";

export function useAppNavigation(): {
  activeRepoId: string | null;
  overviewActive: boolean;
  /** Where a repo tab mounted by the last navigation opens its handset deck */
  deckLanding: DeckLanding;
  selectRepo: (repoId: string) => void;
  openRepoStream: (repoId: string) => void;
  openRepoSource: (repoId: string) => void;
  openOverview: () => void;
} {
  const { config, isLoaded, setLastActiveTabId, setLastActiveGroupRepoId, setMainView, setFilesMode, setActiveRail } = useConfig();
  const [deckLanding, setDeckLanding] = useState<DeckLanding>("files");
  const remembered = config.uiState.lastActiveTabId;
  const activeRepoId = config.repos.some((repo) => repo.repoId === remembered)
    ? remembered : config.repos[0]?.repoId ?? null;

  useEffect(() => {
    if (isLoaded && remembered !== activeRepoId) setLastActiveTabId(activeRepoId);
  }, [activeRepoId, isLoaded, remembered, setLastActiveTabId]);

  const navigate = useCallback((repoId: string, landing: DeckLanding) => {
    setDeckLanding(landing);
    setLastActiveTabId(repoId);
    const repo = config.repos.find((entry) => entry.repoId === repoId);
    if (repo?.groupId) setLastActiveGroupRepoId(repo.groupId, repoId);
    setMainView("repository");
  }, [config.repos, setLastActiveGroupRepoId, setLastActiveTabId, setMainView]);
  const selectRepo = useCallback((repoId: string) => { navigate(repoId, "files"); }, [navigate]);

  const openRepoStream = useCallback((repoId: string) => {
    setFilesMode("stream");
    navigate(repoId, "files");
  }, [navigate, setFilesMode]);
  const openRepoSource = useCallback((repoId: string) => {
    setActiveRail("source");
    navigate(repoId, "rail");
  }, [navigate, setActiveRail]);
  const openOverview = useCallback(() => { setMainView("overview"); }, [setMainView]);

  return {
    activeRepoId, overviewActive: config.uiState.mainView === "overview", deckLanding,
    selectRepo, openRepoStream, openRepoSource, openOverview,
  };
}
