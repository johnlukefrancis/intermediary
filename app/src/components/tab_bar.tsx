// Path: app/src/components/tab_bar.tsx
// Description: Tab navigation component driven by config repos

import type React from "react";
import "../styles/tab_bar.css";

interface TabInfo {
  repoId: string;
  label: string;
}

interface TabBarProps {
  tabs: TabInfo[];
  activeTab: string | null;
  onTabChange: (repoId: string) => void;
}

export function TabBar({ tabs, activeTab, onTabChange }: TabBarProps): React.JSX.Element {
  return (
    <nav className="tab-bar">
      {tabs.map((tab) => (
        <button
          key={tab.repoId}
          className={`tab-button ${activeTab === tab.repoId ? "active" : ""}`}
          onClick={() => {
            onTabChange(tab.repoId);
          }}
          type="button"
          aria-current={activeTab === tab.repoId ? "page" : undefined}
        >
          <span className="tab-label">{tab.label}</span>
        </button>
      ))}
    </nav>
  );
}
