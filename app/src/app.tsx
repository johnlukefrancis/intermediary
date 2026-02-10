// Path: app/src/app.tsx
// Description: Root component with config-driven tab state management

import React, { useState, useCallback, useEffect, useLayoutEffect, useRef, useMemo } from "react";
import { TabBar } from "./components/tab_bar.js";
import { StatusBar } from "./components/status_bar.js";
import { AgentOfflineBanner } from "./components/agent_offline_banner.js";
import { RepoTab } from "./tabs/repo_tab.js";
import { EmptyRepoState } from "./components/empty_repo_state.js";
import { useConfig } from "./hooks/use_config.js";
import { useModeWindowSnap } from "./hooks/use_mode_window_snap.js";
import { useModeWindowBoundsPersistence } from "./hooks/use_mode_window_bounds_persistence.js";
import { useMotionGovernor } from "./hooks/use_motion_governor.js";
import { useStartupReady } from "./hooks/use_startup_ready.js";
import type { RepoConfig, RepoRoot } from "./shared/config.js";
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
  root: RepoRoot;
}

/** A grouped tab containing multiple repos with a dropdown */
export interface GroupTab {
  type: "group";
  groupId: string;
  groupLabel: string;
  repos: Array<{ repoId: string; label: string; root: RepoRoot }>;
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
      group.repos.push({ repoId: repo.repoId, label: repo.label, root: repo.root });
    } else {
      // Standalone repo
      tabs.push({
        type: "single",
        repoId: repo.repoId,
        label: repo.label,
        root: repo.root,
      });
    }
  }

  return tabs;
}

export function App(): React.JSX.Element {
  const {
    config,
    isLoaded,
    setLastActiveTabId,
    setLastActiveGroupRepoId,
    setWindowBoundsForMode,
  } = useConfig();
  const { motionPaused } = useMotionGovernor();

  useModeWindowSnap(config.uiMode, config.uiState.windowBoundsByMode, isLoaded);
  useModeWindowBoundsPersistence(config.uiMode, setWindowBoundsForMode);
  useStartupReady(isLoaded);

  useEffect(() => {
    const root = document.documentElement;
    root.dataset.themeMode = config.themeMode;
    return () => {
      delete root.dataset.themeMode;
    };
  }, [config.themeMode]);

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
          const repo = config.repos.find((entry) => entry.repoId === validRepoId);
          if (repo?.groupId) {
            setLastActiveGroupRepoId(repo.groupId, validRepoId);
          }
        }
      }
    }
  }, [
    isLoaded,
    config.repos,
    activeRepoId,
    config.uiState.lastActiveTabId,
    validateRepoId,
    setLastActiveTabId,
    setLastActiveGroupRepoId,
  ]);

  // Wrap setter to also persist
  const setActiveRepoId = useCallback(
    (repoId: string) => {
      setActiveRepoIdState(repoId);
      setLastActiveTabId(repoId);
      const repo = config.repos.find((entry) => entry.repoId === repoId);
      if (repo?.groupId) {
        setLastActiveGroupRepoId(repo.groupId, repoId);
      }
    },
    [setLastActiveTabId, setLastActiveGroupRepoId, config.repos]
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

  // Expose header height as CSS variable for overlay positioning
  const appRef = useRef<HTMLDivElement>(null);
  const headerRef = useRef<HTMLElement>(null);

  useLayoutEffect(() => {
    const header = headerRef.current;
    const app = appRef.current;
    if (!header || !app) return;

    const ro = new ResizeObserver(([entry]) => {
      if (entry) {
        app.style.setProperty(
          "--header-stack-height",
          `${entry.contentRect.height}px`
        );
      }
    });
    ro.observe(header);
    return () => { ro.disconnect(); };
  }, []);

  // Empty state: no repos configured
  if (config.repos.length === 0) {
    return (
      <div
        ref={appRef}
        className="app"
        data-motion={motionPaused ? "paused" : undefined}
        data-theme-mode={config.themeMode}
        data-ui-mode={config.uiMode}
        style={themeStyle}
      >
        <header ref={headerRef} className="header-stack glass-surface">
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
      ref={appRef}
      className="app"
      data-active-tab={activeRepoId}
      data-motion={motionPaused ? "paused" : undefined}
      data-theme-mode={config.themeMode}
      data-ui-mode={config.uiMode}
      style={themeStyle}
    >
      <header ref={headerRef} className="header-stack glass-surface">
        <TabBar
          tabs={tabs}
          activeRepoId={activeRepoId}
          tabThemes={config.tabThemes}
          lastActiveGroupRepoIds={config.uiState.lastActiveGroupRepoIds}
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
