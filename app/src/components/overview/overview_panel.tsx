// Path: app/src/components/overview/overview_panel.tsx
// Description: Full-width chronological work feed with active repository chips and watch-health awareness

import type React from "react";
import { useMemo, useState } from "react";
import type { RepoConfig, TabTheme } from "../../shared/config.js";
import type { OverviewSnapshot } from "../../lib/overview/overview_types.js";
import { isWorking } from "../../lib/overview/overview_focus.js";
import { DEFAULT_ACCENT_HEX, hexToAccentCssVars } from "../../lib/theme/accent_utils.js";
import { useOverviewImages } from "../../hooks/overview/use_overview_images.js";
import { useOverviewFileActions } from "../../hooks/overview/use_overview_file_actions.js";
import { ContextMenu } from "../context_menu.js";
import { DragErrorNotice } from "../drag_error_notice.js";
import { OverviewFeed } from "./overview_feed.js";
import { overviewFeedEntries } from "./overview_feed_entries.js";
import { monitoringSummary } from "./overview_presentation.js";
import { EmptyStatePanel } from "../empty_state_panel.js";
import "../../styles/overview/overview_panel.css";
import "../../styles/overview/overview_feed.css";
import "../../styles/overview/overview_detail.css";

export interface OverviewPanelProps {
  repos: readonly RepoConfig[];
  tabThemes: Readonly<Record<string, TabTheme>>;
  snapshot: OverviewSnapshot;
  now: number;
  documentHidden: boolean;
  onOpenRepo: (repoId: string) => void;
  onOpenFile: (repo: RepoConfig, path: string) => void;
}

export function OverviewPanel({ repos, tabThemes, snapshot, now, documentHidden, onOpenRepo, onOpenFile }: OverviewPanelProps): React.JSX.Element {
  const [selected, setSelected] = useState<string | null>(null);
  const working = useMemo(() => repos.filter((repo) => isWorking(snapshot.repos.get(repo.repoId), now)), [repos, snapshot, now]);
  const selection = working.some((repo) => repo.repoId === selected) ? selected : null;
  const shown = useMemo(() => selection === null ? working : working.filter((repo) => repo.repoId === selection), [selection, working]);
  const entries = useMemo(() => overviewFeedEntries(shown, snapshot, now), [shown, snapshot, now]);
  const images = useOverviewImages(entries, documentHidden);
  const actions = useOverviewFileActions(repos, documentHidden);
  const monitoring = monitoringSummary(repos, snapshot);
  return (
    <section className={entries.length === 0 ? "overview-panel" : "panel overview-panel"} aria-label="Repository Overview" data-empty={entries.length === 0}>
      {(working.length > 0 || monitoring.issues > 0) && <div className="overview-toolbar">
        <div className="overview-repositories" role="group" aria-label="Filter active repositories">
          {working.length > 0 && <button type="button" className="overview-repo-chip" aria-pressed={selection === null} onClick={() => { setSelected(null); }}
            title="Show every active repository">All</button>}
          {working.map((repo) => <button key={repo.repoId} type="button" className="overview-repo-chip"
            style={hexToAccentCssVars(tabThemes[repo.groupId ?? repo.repoId]?.accentHex ?? DEFAULT_ACCENT_HEX) as React.CSSProperties}
            aria-pressed={selection === repo.repoId} onClick={() => { setSelected(repo.repoId); }}
            title={`${repo.root.kind.toUpperCase()} · ${repo.root.path}`}>
            <span className="overview-repo-chip__dot" aria-hidden="true" /><span className="overview-repo-chip__label">{repo.label}</span></button>)}
        </div>
        {monitoring.issues > 0 && <span className="badge badge--warning overview-monitoring" title={monitoring.detail}
          aria-label={`${String(monitoring.issues)} monitoring issues`}>{monitoring.issues === 1 ? "1 ISSUE" : `${String(monitoring.issues)} ISSUES`}</span>}
      </div>}
      {actions.dragState.error && <DragErrorNotice message={actions.dragState.error} onDismiss={actions.clearError} />}
      {entries.length > 0 ? <OverviewFeed entries={entries} images={images} tabThemes={tabThemes} now={now} documentHidden={documentHidden}
        viewKey={selection ?? "all"} onOpenRepo={onOpenRepo} onOpenFile={onOpenFile} onDragFile={actions.dragFile} onContextMenu={actions.openMenu} /> : <EmptyStatePanel
          title={monitoring.emptyHeading} description={monitoring.emptyDetail} />}
      {actions.menu && <ContextMenu x={actions.menu.x} y={actions.menu.y} items={actions.menu.items} onClose={actions.closeMenu} />}
    </section>
  );
}
