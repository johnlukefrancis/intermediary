// Path: app/src/components/tab_bar/overview_tab.tsx
// Description: Global activity destination using the repository navigation grammar

import type React from "react";

export function OverviewTab({ active, onSelect }: { active: boolean; onSelect: () => void }): React.JSX.Element {
  return <button className={`tab-button tab-overview${active ? " active" : ""}`} type="button"
    onClick={onSelect} aria-current={active ? "page" : undefined}
    title="Overview — activity across all repositories">
    <svg width="14" height="14" viewBox="0 0 20 20" fill="none" aria-hidden="true">
      <rect x="2" y="2" width="6" height="6" rx="1" stroke="currentColor" />
      <rect x="12" y="2" width="6" height="6" rx="1" stroke="currentColor" />
      <rect x="2" y="12" width="6" height="6" rx="1" stroke="currentColor" />
      <rect x="12" y="12" width="6" height="6" rx="1" stroke="currentColor" />
    </svg>
    <span>Overview</span>
  </button>;
}
