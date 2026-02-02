// Path: app/src/app.tsx
// Description: Root component with config-driven tab state management

import React, { useState, useCallback, useEffect, useMemo } from "react";
import { TabBar } from "./components/tab_bar.js";
import { StatusBar } from "./components/status_bar.js";
import { RepoTab } from "./tabs/repo_tab.js";
import { EmptyRepoState } from "./components/empty_repo_state.js";
import { useConfig } from "./hooks/use_config.js";
import type { RepoConfig } from "./shared/config.js";

/** A standalone repo tab */
export interface SingleTab {
  type: "single";
  repoId: string;
  label: string;
}

/** A grouped tab containing multiple repos with a dropdown */
export interface GroupTab {
  type: "group";
  groupId: string;
  groupLabel: string;
  repos: Array<{ repoId: string; label: string }>;
}

export type TabItem = SingleTab | GroupTab;

/** Derive tabs from repos, grouping by groupId */
function deriveTabsFromRepos(repos: RepoConfig[]): TabItem[] {
  const groupMap = new Map<string, GroupTab>();
  const tabs: TabItem[] = [];

  for (const repo of repos) {
    if (repo.groupId) {
      // Grouped repo - groupLabel is optional, fallback to groupId
      let group = groupMap.get(repo.groupId);
      if (!group) {
        group = {
          type: "group",
          groupId: repo.groupId,
          groupLabel: repo.groupLabel ?? repo.groupId,
          repos: [],
        };
        groupMap.set(repo.groupId, group);
        tabs.push(group);
      }
      // Update groupLabel if this repo has one and current label is the fallback
      if (repo.groupLabel && group.groupLabel === group.groupId) {
        group.groupLabel = repo.groupLabel;
      }
      group.repos.push({ repoId: repo.repoId, label: repo.label });
    } else {
      // Standalone repo
      tabs.push({
        type: "single",
        repoId: repo.repoId,
        label: repo.label,
      });
    }
  }

  return tabs;
}

export function App(): React.JSX.Element {
  const { config, isLoaded, setLastActiveTabId } = useConfig();

  // Derive tabs with grouping from config repos
  const tabs = useMemo(() => deriveTabsFromRepos(config.repos), [config.repos]);

  // Get valid repoIds for validation
  const validRepoIds = useMemo(
    () => new Set(config.repos.map((r) => r.repoId)),
    [config.repos]
  );

  // Determine initial/default tab (first repo, or null if none)
  const defaultRepoId = config.repos[0]?.repoId ?? null;

  // Validate that a repoId exists in current config
  const validateRepoId = useCallback(
    (repoId: string | null): string | null => {
      if (repoId && validRepoIds.has(repoId)) return repoId;
      return defaultRepoId;
    },
    [validRepoIds, defaultRepoId]
  );

  // Initialize activeRepoId from persisted config with validation
  const [activeRepoId, setActiveRepoIdState] = useState<string | null>(() => {
    return validateRepoId(config.uiState.lastActiveTabId);
  });

  // Update local state when config loads or repos change
  useEffect(() => {
    if (isLoaded) {
      // Validate current activeRepoId against current repos
      const validRepoId = validateRepoId(activeRepoId ?? config.uiState.lastActiveTabId);
      if (validRepoId !== activeRepoId) {
        setActiveRepoIdState(validRepoId);
        if (validRepoId) {
          setLastActiveTabId(validRepoId);
        }
      }
    }
  }, [isLoaded, config.repos, activeRepoId, config.uiState.lastActiveTabId, validateRepoId, setLastActiveTabId]);

  // Wrap setter to also persist
  const setActiveRepoId = useCallback(
    (repoId: string) => {
      setActiveRepoIdState(repoId);
      setLastActiveTabId(repoId);
    },
    [setLastActiveTabId]
  );

  // Handle new repo added - auto-select it
  const handleRepoAdded = useCallback(
    (repoId: string) => {
      setActiveRepoId(repoId);
    },
    [setActiveRepoId]
  );

  // Empty state: no repos configured
  if (config.repos.length === 0) {
    return (
      <div className="app">
        <header className="header-stack glass-surface">
          <StatusBar />
        </header>
        <main className="tab-content">
          <EmptyRepoState onRepoAdded={handleRepoAdded} />
        </main>
      </div>
    );
  }

  return (
    <div className="app" data-active-tab={activeRepoId}>
      <header className="header-stack glass-surface">
        <TabBar
          tabs={tabs}
          activeRepoId={activeRepoId}
          onRepoChange={setActiveRepoId}
          onRepoAdded={handleRepoAdded}
        />
        <StatusBar />
      </header>
      <main className="tab-content">
        {activeRepoId && <RepoTab repoId={activeRepoId} />}
      </main>
    </div>
  );
}
