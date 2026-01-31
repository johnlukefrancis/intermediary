// Path: app/src/components/tab_bar.tsx
// Description: Tab navigation component

import type React from "react";
import type { TabId } from "../shared/protocol";
import "../styles/tab_bar.css";

interface TabBarProps {
  activeTab: TabId;
  onTabChange: (tab: TabId) => void;
}

const TABS: { id: TabId; label: string }[] = [
  { id: "texture-portal", label: "TexturePortal" },
  { id: "triangle-rain", label: "Triangle Rain" },
  { id: "intermediary", label: "Intermediary" },
];

export function TabBar({ activeTab, onTabChange }: TabBarProps): React.JSX.Element {
  return (
    <nav className="tab-bar">
      {TABS.map((tab) => (
        <button
          key={tab.id}
          className={`tab-button ${activeTab === tab.id ? "active" : ""}`}
          onClick={() => {
            onTabChange(tab.id);
          }}
          type="button"
          aria-current={activeTab === tab.id ? "page" : undefined}
        >
          <span className="tab-label">{tab.label}</span>
        </button>
      ))}
    </nav>
  );
}
