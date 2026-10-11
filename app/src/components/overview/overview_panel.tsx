// Path: app/src/components/overview/overview_panel.tsx
// Description: Overview chronological work feed for the shown repositories, with its quiet-state empty panel

import type React from "react";
import { useMemo } from "react";
import type { RepoConfig, TabTheme } from "../../shared/config.js";
import type { OverviewSnapshot } from "../../lib/overview/overview_types.js";
import { useOverviewImages } from "../../hooks/overview/use_overview_images.js";
import { useOverviewFileActions } from "../../hooks/overview/use_overview_file_actions.js";
import { ContextMenu } from "../context_menu.js";
import { DragErrorNotice } from "../drag_error_notice.js";
import { OverviewFeed } from "./overview_feed.js";
import { overviewFeedEntries } from "./overview_feed_entries.js";
import { EmptyStatePanel } from "../empty_state_panel.js";
import "../../styles/overview/overview_panel.css";
import "../../styles/overview/overview_feed.css";
import "../../styles/overview/overview_detail.css";

export interface OverviewPanelProps {
  repos: readonly RepoConfig[];
  shown: readonly RepoConfig[];
  selection: string | null;
  tabThemes: Readonly<Record<string, TabTheme>>;
  snapshot: OverviewSnapshot;
  now: number;
  documentHidden: boolean;
  emptyCopy: { heading: string; detail: string };
  onOpenRepo: (repoId: string) => void;
  onOpenFile: (repo: RepoConfig, path: string) => void;
}

export function OverviewPanel({ repos, shown, selection, tabThemes, snapshot, now, documentHidden, emptyCopy, onOpenRepo, onOpenFile }: OverviewPanelProps): React.JSX.Element {
  const entries = useMemo(() => overviewFeedEntries(shown, snapshot, now), [shown, snapshot, now]);
  const images = useOverviewImages(entries, documentHidden);
  const actions = useOverviewFileActions(repos, documentHidden);
  return (
    <section className={entries.length === 0 ? "overview-panel" : "panel overview-panel"} aria-label="Repository Overview" data-empty={entries.length === 0}>
      {actions.dragState.error && <DragErrorNotice message={actions.dragState.error} onDismiss={actions.clearError} />}
      {entries.length > 0 ? <OverviewFeed entries={entries} images={images} tabThemes={tabThemes} now={now} documentHidden={documentHidden}
        viewKey={selection ?? "all"} onOpenRepo={onOpenRepo} onOpenFile={onOpenFile} onDragFile={actions.dragFile} onContextMenu={actions.openMenu} /> : <EmptyStatePanel
          title={emptyCopy.heading} description={emptyCopy.detail} />}
      {actions.menu && <ContextMenu x={actions.menu.x} y={actions.menu.y} items={actions.menu.items} onClose={actions.closeMenu} />}
    </section>
  );
}
