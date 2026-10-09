// Path: app/src/app.tsx
// Description: Root component with config-driven tab state management

import React, { useEffect, useLayoutEffect, useRef, useMemo } from "react";
import { TabBar } from "./components/tab_bar.js";
import { StatusBar } from "./components/status_bar.js";
import { AgentOfflineBanner } from "./components/agent_offline_banner.js";
import { RepoTab } from "./tabs/repo_tab.js";
import { EmptyRepoState } from "./components/empty_repo_state.js";
import { OverviewView } from "./components/overview_view.js";
import { useActivityHost } from "./hooks/use_activity_host.js";
import { useAppNavigation } from "./hooks/use_app_navigation.js";
import { useConfig } from "./hooks/use_config.js";
import { useEffectiveUiMode } from "./hooks/use_effective_ui_mode.js";
import { useModeWindowSnap } from "./hooks/use_mode_window_snap.js";
import { useModeWindowBoundsPersistence } from "./hooks/use_mode_window_bounds_persistence.js";
import { useMotionGovernor } from "./hooks/use_motion_governor.js";
import { useStartupReady } from "./hooks/use_startup_ready.js";
import { useTerminalLifecycle } from "./hooks/terminal/use_terminal_lifecycle.js";
import { deriveTabsFromRepos } from "./lib/tabs/tab_items.js";
import {
  hexToAccentCssVars,
  DEFAULT_ACCENT_HEX,
} from "./lib/theme/accent_utils.js";
import { resolveTextureUrl } from "./lib/theme/texture_catalog.js";
import type { RepoRoot } from "./shared/config.js";

export function App(): React.JSX.Element {
  const {
    config,
    isLoaded,
    persistenceLocked,
    persistenceLockReason,
    setWindowBoundsForMode,
  } = useConfig();
  const { motionPaused, documentHidden } = useMotionGovernor();
  const effectiveUiMode = useEffectiveUiMode(config.uiMode, isLoaded);
  const { activeRepoId, overviewActive, selectRepo, openRepoStream, openOverview } = useAppNavigation();
  useActivityHost(documentHidden);

  useModeWindowSnap(config.uiMode, config.uiState.windowBoundsByMode, isLoaded);
  useModeWindowBoundsPersistence(effectiveUiMode, setWindowBoundsForMode);
  useStartupReady(isLoaded);
  const windowOpacityAlpha = useMemo(
    (): number => Math.max(0, Math.min(100, config.windowOpacityPercent)) / 100,
    [config.windowOpacityPercent]
  );

  useEffect(() => {
    const root = document.documentElement;
    root.dataset.themeMode = config.themeMode;
    root.dataset.uiMode = effectiveUiMode;
    root.style.setProperty(
      "--window-opacity-percent",
      `${config.windowOpacityPercent}`
    );
    root.style.setProperty("--window-opacity-alpha", `${windowOpacityAlpha}`);
    root.style.setProperty(
      "--texture-intensity-percent",
      `${config.textureIntensityPercent}`
    );
    return () => {
      delete root.dataset.themeMode;
      delete root.dataset.uiMode;
      root.style.removeProperty("--window-opacity-percent");
      root.style.removeProperty("--window-opacity-alpha");
      root.style.removeProperty("--texture-intensity-percent");
    };
  }, [
    config.themeMode,
    config.textureIntensityPercent,
    config.windowOpacityPercent,
    effectiveUiMode,
    windowOpacityAlpha,
  ]);

  // Derive tabs with grouping from config repos
  const tabs = useMemo(() => deriveTabsFromRepos(config.repos), [config.repos]);

  // Project both repo membership and root identity to the terminal lifecycle. A reused repoId with
  // a different root must close its old terminal group before any tab can be restarted.
  const configuredRepoRoots = useMemo(
    (): ReadonlyMap<string, RepoRoot> =>
      new Map(config.repos.map((repo) => [repo.repoId, repo.root] as const)),
    [config.repos]
  );

  // Terminal groups outlive view changes; removing a configured root closes its shells.
  useTerminalLifecycle(configuredRepoRoots, !motionPaused);

  // Compute theme key: groupId if repo is grouped, else repoId
  const activeThemeKey = useMemo((): string | null => {
    if (overviewActive || !activeRepoId) return null;
    const activeRepo = config.repos.find((r) => r.repoId === activeRepoId);
    if (!activeRepo) return null;
    return activeRepo.groupId ?? activeRepoId;
  }, [activeRepoId, config.repos, overviewActive]);

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
        data-visibility={documentHidden ? "hidden" : "visible"}
        data-theme-mode={config.themeMode}
        data-ui-mode={effectiveUiMode}
        style={themeStyle}
      >
        <header ref={headerRef} className="header-stack glass-surface">
          <AgentOfflineBanner />
          <StatusBar />
        </header>
        <main className="tab-content">
          <EmptyRepoState
            onRepoAdded={selectRepo}
            persistenceLocked={persistenceLocked}
            persistenceLockReason={persistenceLockReason}
          />
        </main>
      </div>
    );
  }

  return (
    <div
      ref={appRef}
      className="app"
      data-active-tab={activeRepoId}
      data-main-view={overviewActive ? "overview" : "repository"}
      data-motion={motionPaused ? "paused" : undefined}
      data-visibility={documentHidden ? "hidden" : "visible"}
      data-theme-mode={config.themeMode}
      data-ui-mode={effectiveUiMode}
      style={themeStyle}
    >
      <header ref={headerRef} className="header-stack glass-surface">
        <TabBar
          tabs={tabs}
          activeRepoId={overviewActive ? null : activeRepoId}
          overviewActive={overviewActive}
          onOverview={openOverview}
          tabThemes={config.tabThemes}
          lastActiveGroupRepoIds={config.uiState.lastActiveGroupRepoIds}
          onRepoChange={selectRepo}
          onRepoAdded={selectRepo}
        />
        <AgentOfflineBanner />
        <StatusBar />
      </header>
      <main className="tab-content">
        {overviewActive
          ? <OverviewView documentHidden={documentHidden} isHandset={effectiveUiMode === "handset"} onOpenRepo={openRepoStream} />
          : activeRepoId && <RepoTab repoId={activeRepoId} uiMode={effectiveUiMode} />}
      </main>
    </div>
  );
}
