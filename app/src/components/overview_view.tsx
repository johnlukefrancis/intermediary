// Path: app/src/components/overview_view.tsx
// Description: Connects Overview's feed and Git column to configured roots, the shared filter and the full-pane workspace

import { useCallback, useEffect, useRef, useState, useSyncExternalStore, type JSX } from "react";
import { useConfig } from "../hooks/use_config.js";
import { useOverview } from "../hooks/overview/use_overview.js";
import { useOverviewSelection } from "../hooks/overview/use_overview_selection.js";
import type { RepoConfig } from "../shared/config.js";
import type { SourceControlEntry } from "../shared/protocol.js";
import type { OverviewSnapshot } from "../lib/overview/overview_types.js";
import { overviewLanes } from "../lib/overview/overview_lanes.js";
import { ThreeColumn } from "./layout/three_column.js";
import { OverviewPanel } from "./overview/overview_panel.js";
import { OverviewToolbar } from "./overview/overview_toolbar.js";
import { OverviewHandsetDeck } from "./overview/overview_handset_deck.js";
import { OverviewGitColumn } from "./overview/git/overview_git_column.js";
import { monitoringSummary } from "./overview/overview_presentation.js";
import { OverviewWorkspace, type OverviewTarget } from "./overview/overview_workspace.js";
import "../styles/overview/overview_view.css";

interface Opened { repo: RepoConfig; target: OverviewTarget; snapshot: OverviewSnapshot; now: number }

function openedKey({ repo, target }: Opened): string {
  const entry = target.kind === "diff" ? target.entry : null;
  return JSON.stringify([repo.repoId, repo.root, target.kind, entry?.area ?? null,
    entry?.path ?? (target.kind === "file" ? target.path : null), entry?.originalPath ?? null]);
}

export function OverviewView({ documentHidden, isHandset, onOpenRepo, onOpenRepoSource }: {
  documentHidden: boolean;
  isHandset: boolean;
  onOpenRepo: (repoId: string) => void;
  onOpenRepoSource: (repoId: string) => void;
}): JSX.Element {
  const { config, setRailWidthPercent } = useConfig();
  const [opened, setOpened] = useState<Opened | null>(null);
  const currentRepo = opened === null ? undefined : config.repos.find((repo) => repo.repoId === opened.repo.repoId);
  const target = opened !== null && currentRepo?.root.kind === opened.repo.root.kind
    && currentRepo.root.path === opened.repo.root.path ? opened : null;
  const feedHidden = documentHidden || target !== null;
  const { snapshot, now } = useOverview(feedHidden);
  const lanes = useSyncExternalStore(overviewLanes.subscribe, overviewLanes.getSnapshot);
  const latest = useRef({ snapshot, now });
  useEffect(() => { latest.current = { snapshot, now }; }, [snapshot, now]);
  const feedSnapshot = target?.snapshot ?? snapshot;
  const feedNow = target?.now ?? now;
  const { working, selection, shown, select } = useOverviewSelection(config.repos, feedSnapshot, feedNow, lanes);
  const monitoring = monitoringSummary(config.repos, feedSnapshot);

  const openFile = useCallback((repo: RepoConfig, path: string) => {
    setOpened({ repo, target: { kind: "file", path }, ...latest.current });
  }, []);
  const openDiff = useCallback((repo: RepoConfig, entry: SourceControlEntry) => {
    setOpened({ repo, target: { kind: "diff", entry }, ...latest.current });
  }, []);
  const closeTarget = useCallback(() => { setOpened(null); }, []);

  useEffect(() => {
    if (opened !== null && target === null) setOpened(null);
  }, [opened, target]);

  const feed = <OverviewPanel repos={config.repos} shown={shown} selection={selection} tabThemes={config.tabThemes}
    snapshot={feedSnapshot} now={feedNow} documentHidden={feedHidden}
    emptyCopy={{ heading: monitoring.emptyHeading, detail: monitoring.emptyDetail }}
    onOpenRepo={onOpenRepo} onOpenFile={openFile} />;
  const git = working.length === 0 ? null : <OverviewGitColumn working={working} shown={shown} tabThemes={config.tabThemes}
    snapshot={feedSnapshot} lanes={lanes} now={feedNow} onOpenRepoSource={onOpenRepoSource} onOpenDiff={openDiff} />;
  const toolbar = (working.length > 0 || monitoring.issues > 0) && <OverviewToolbar working={working} selection={selection}
    tabThemes={config.tabThemes} monitoring={monitoring} onSelect={select} />;
  const decked = git !== null && isHandset;
  const body = git === null ? feed : decked ? <OverviewHandsetDeck shown={shown} toolbar={toolbar} feed={feed} git={git} />
    : <ThreeColumn variant="files" railWidthPercent={config.uiState.railWidthPercent}
      onRailWidthChange={setRailWidthPercent} fileContent={feed} railContent={git} />;

  return <div className="overview-view">
    <div className="overview-view__feed" hidden={target !== null}>
      {!decked && toolbar}
      {body}
    </div>
    {target !== null && <OverviewWorkspace key={openedKey(target)} repo={currentRepo ?? target.repo}
      target={target.target} isHandset={isHandset} onClose={closeTarget} />}
  </div>;
}
