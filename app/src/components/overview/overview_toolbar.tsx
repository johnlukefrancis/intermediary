// Path: app/src/components/overview/overview_toolbar.tsx
// Description: Overview glass strip above both columns: active-repository filter chips and the monitoring badge

import type React from "react";
import type { RepoConfig, TabTheme } from "../../shared/config.js";
import { DEFAULT_ACCENT_HEX, hexToAccentCssVars } from "../../lib/theme/accent_utils.js";

interface OverviewToolbarProps {
  working: readonly RepoConfig[];
  selection: string | null;
  tabThemes: Readonly<Record<string, TabTheme>>;
  monitoring: { issues: number; detail: string };
  onSelect: (repoId: string | null) => void;
}

export function OverviewToolbar({ working, selection, tabThemes, monitoring, onSelect }: OverviewToolbarProps): React.JSX.Element {
  return (
    <div className="overview-toolbar">
      <div className="overview-repositories" role="group" aria-label="Filter active repositories">
        {working.length > 0 && <button type="button" className="overview-repo-chip" aria-pressed={selection === null} onClick={() => { onSelect(null); }}
          title="Show every active repository">All</button>}
        {working.map((repo) => <button key={repo.repoId} type="button" className="overview-repo-chip"
          style={hexToAccentCssVars(tabThemes[repo.groupId ?? repo.repoId]?.accentHex ?? DEFAULT_ACCENT_HEX) as React.CSSProperties}
          aria-pressed={selection === repo.repoId} onClick={() => { onSelect(repo.repoId); }}
          title={`${repo.root.kind.toUpperCase()} · ${repo.root.path}`}>
          <span className="overview-repo-chip__dot" aria-hidden="true" /><span className="overview-repo-chip__label">{repo.label}</span></button>)}
      </div>
      {monitoring.issues > 0 && <span className="badge badge--warning overview-monitoring" title={monitoring.detail}
        aria-label={`${String(monitoring.issues)} monitoring issues`}>{monitoring.issues === 1 ? "1 ISSUE" : `${String(monitoring.issues)} ISSUES`}</span>}
    </div>
  );
}
