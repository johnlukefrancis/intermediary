// Path: app/src/app.tsx
// Description: Root component with tab state management

import React, { useState, useCallback, useEffect } from "react";
import { TabBar } from "./components/tab_bar.js";
import { StatusBar } from "./components/status_bar.js";
import { TexturePortalTab } from "./tabs/texture_portal_tab.js";
import { TriangleRainTab } from "./tabs/triangle_rain_tab.js";
import { IntermediaryTab } from "./tabs/intermediary_tab.js";
import { useConfig } from "./hooks/use_config.js";
import type { TabId } from "./shared/ids.js";

export function App(): React.JSX.Element {
  const { config, isLoaded, setLastActiveTabId } = useConfig();

  // Initialize tab from persisted config or default to "intermediary"
  const [activeTab, setActiveTabState] = useState<TabId>(() => {
    return config.uiState.lastActiveTabId ?? "intermediary";
  });

  // Update local state when config loads (handles async load)
  useEffect(() => {
    if (isLoaded && config.uiState.lastActiveTabId) {
      setActiveTabState(config.uiState.lastActiveTabId);
    }
  }, [isLoaded, config.uiState.lastActiveTabId]);

  // Wrap setActiveTab to also persist
  const setActiveTab = useCallback(
    (tabId: TabId) => {
      setActiveTabState(tabId);
      setLastActiveTabId(tabId);
    },
    [setLastActiveTabId]
  );

  return (
    <div className="app" data-active-tab={activeTab}>
      <header className="header-stack glass-surface">
        <TabBar activeTab={activeTab} onTabChange={setActiveTab} />
        <StatusBar />
      </header>
      <main className="tab-content">
        {activeTab === "texture-portal" && <TexturePortalTab />}
        {activeTab === "triangle-rain" && <TriangleRainTab />}
        {activeTab === "intermediary" && <IntermediaryTab />}
      </main>
    </div>
  );
}
