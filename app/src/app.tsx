// Path: app/src/app.tsx
// Description: Root component with config-driven tab state management

import React, { useState, useCallback, useEffect, useMemo } from "react";
import { TabBar } from "./components/tab_bar.js";
import { StatusBar } from "./components/status_bar.js";
import { AgentOfflineBanner } from "./components/agent_offline_banner.js";
import { RepoTab } from "./tabs/repo_tab.js";
import { EmptyRepoState } from "./components/empty_repo_state.js";
import { useConfig } from "./hooks/use_config.js";
import { useMotionGovernor } from "./hooks/use_motion_governor.js";
import type { RepoConfig } from "./shared/config.js";
import {
  hexToAccentCssVars,
  DEFAULT_ACCENT_HEX,
} from "./lib/theme/accent_utils.js";
import { resolveTextureUrl } from "./lib/theme/texture_catalog.js";

/** A standalone repo tab */
export interface SingleTab {
  type: "single";
  repoId: string;
  label: string;
  wslPath: string;
}

/** A grouped tab containing multiple repos with a dropdown */
export interface GroupTab {
  type: "group";
  groupId: string;
  groupLabel: string;
  repos: Array<{ repoId: string; label: string; wslPath: string }>;
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
      group.repos.push({ repoId: repo.repoId, label: repo.label, wslPath: repo.wslPath });
    } else {
      // Standalone repo
      tabs.push({
        type: "single",
        repoId: repo.repoId,
        label: repo.label,
        wslPath: repo.wslPath,
      });
    }
  }

  return tabs;
}

export function App(): React.JSX.Element {
  const { config, isLoaded, setLastActiveTabId } = useConfig();
  const { motionPaused } = useMotionGovernor();

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

  // Compute theme key: groupId if repo is grouped, else repoId
  const activeThemeKey = useMemo((): string | null => {
    if (!activeRepoId) return null;
    const activeRepo = config.repos.find((r) => r.repoId === activeRepoId);
    if (!activeRepo) return null;
    return activeRepo.groupId ?? activeRepoId;
  }, [activeRepoId, config.repos]);

  // Get accent color from config or use default
  const accentHex = useMemo((): string => {
    if (!activeThemeKey) return DEFAULT_ACCENT_HEX;
    return config.tabThemes[activeThemeKey]?.accentHex ?? DEFAULT_ACCENT_HEX;
  }, [activeThemeKey, config.tabThemes]);

  // Compute CSS variables as inline style
  const accentStyle = useMemo(
    (): React.CSSProperties => hexToAccentCssVars(accentHex) as React.CSSProperties,
    [accentHex]
  );
  const textureUrl = useMemo((): string | null => {
    if (!activeThemeKey) return resolveTextureUrl(undefined);
    return resolveTextureUrl(config.tabThemes[activeThemeKey]?.textureId);
  }, [activeThemeKey, config.tabThemes]);

  const themeStyle = useMemo<React.CSSProperties>(
    () => ({
      ...accentStyle,
      "--deck-texture-url": textureUrl ? `url("${textureUrl}")` : "none",
    }),
    [accentStyle, textureUrl]
  );

  // Empty state: no repos configured
  if (config.repos.length === 0) {
    return (
      <div
        className="app"
        data-motion={motionPaused ? "paused" : undefined}
        data-theme-mode={config.themeMode}
        style={themeStyle}
      >
        <header className="header-stack glass-surface">
          <AgentOfflineBanner />
          <StatusBar />
        </header>
        <main className="tab-content">
          <EmptyRepoState onRepoAdded={handleRepoAdded} />
        </main>
      </div>
    );
  }

  return (
    <div
      className="app"
      data-active-tab={activeRepoId}
      data-motion={motionPaused ? "paused" : undefined}
      data-theme-mode={config.themeMode}
      style={themeStyle}
    >
      <header className="header-stack glass-surface">
        <TabBar
          tabs={tabs}
          activeRepoId={activeRepoId}
          onRepoChange={setActiveRepoId}
          onRepoAdded={handleRepoAdded}
        />
        <AgentOfflineBanner />
        <StatusBar />
      </header>
      <main className="tab-content">
        {activeRepoId && <RepoTab repoId={activeRepoId} />}
      </main>
    </div>
  );
}
