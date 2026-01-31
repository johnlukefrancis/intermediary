// Path: app/src/app.tsx
// Description: Root component with config-driven tab state management

import React, { useState, useCallback, useEffect, useMemo } from "react";
import { TabBar } from "./components/tab_bar.js";
import { StatusBar } from "./components/status_bar.js";
import { RepoTab } from "./tabs/repo_tab.js";
import { useConfig } from "./hooks/use_config.js";

export function App(): React.JSX.Element {
  const { config, isLoaded, setLastActiveTabId } = useConfig();

  // Derive tabs from config repos
  const tabs = useMemo(
    () => config.repos.map((r) => ({ repoId: r.repoId, label: r.label })),
    [config.repos]
  );

  // Get valid repoIds for validation
  const validRepoIds = useMemo(
    () => new Set(tabs.map((t) => t.repoId)),
    [tabs]
  );

  // Determine initial/default tab (first repo, or null if none)
  const defaultTab = tabs[0]?.repoId ?? null;

  // Validate that a repoId exists in current config
  const validateRepoId = useCallback(
    (repoId: string | null): string | null => {
      if (repoId && validRepoIds.has(repoId)) return repoId;
      return defaultTab;
    },
    [validRepoIds, defaultTab]
  );

  // Initialize tab from persisted config with validation
  const [activeTab, setActiveTabState] = useState<string | null>(() => {
    return validateRepoId(config.uiState.lastActiveTabId);
  });

  // Update local state when config loads (handles async load)
  useEffect(() => {
    if (isLoaded) {
      const validTab = validateRepoId(config.uiState.lastActiveTabId);
      setActiveTabState(validTab);
    }
  }, [isLoaded, config.uiState.lastActiveTabId, validateRepoId]);

  // Wrap setActiveTab to also persist
  const setActiveTab = useCallback(
    (repoId: string) => {
      setActiveTabState(repoId);
      setLastActiveTabId(repoId);
    },
    [setLastActiveTabId]
  );

  return (
    <div className="app" data-active-tab={activeTab}>
      <header className="header-stack glass-surface">
        <TabBar
          tabs={tabs}
          activeTab={activeTab}
          onTabChange={setActiveTab}
        />
        <StatusBar />
      </header>
      <main className="tab-content">
        {activeTab && <RepoTab repoId={activeTab} />}
      </main>
    </div>
  );
}
