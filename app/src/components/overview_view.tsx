// Path: app/src/components/overview_view.tsx
// Description: Connects the Overview presentation to configured roots and its live projection

import { useCallback, useEffect, useState, type JSX } from "react";
import { useConfig } from "../hooks/use_config.js";
import { useOverview } from "../hooks/overview/use_overview.js";
import type { RepoConfig } from "../shared/config.js";
import type { OverviewSnapshot } from "../lib/overview/overview_types.js";
import { OverviewPanel } from "./overview/overview_panel.js";
import { OverviewWorkspace } from "./overview/overview_workspace.js";
import "../styles/overview/overview_view.css";

interface OpenedFile { repo: RepoConfig; path: string; snapshot: OverviewSnapshot; now: number }

export function OverviewView({ documentHidden, isHandset, onOpenRepo }: {
  documentHidden: boolean;
  isHandset: boolean;
  onOpenRepo: (repoId: string) => void;
}): JSX.Element {
  const { config } = useConfig();
  const [opened, setOpened] = useState<OpenedFile | null>(null);
  const currentRepo = opened === null ? undefined : config.repos.find((repo) => repo.repoId === opened.repo.repoId);
  const target = opened !== null && currentRepo?.root.kind === opened.repo.root.kind
    && currentRepo.root.path === opened.repo.root.path ? opened : null;
  const feedHidden = documentHidden || target !== null;
  const { snapshot, now } = useOverview(feedHidden);
  const openFile = useCallback((repo: RepoConfig, path: string) => {
    setOpened({ repo, path, snapshot, now });
  }, [snapshot, now]);
  const closeFile = useCallback(() => { setOpened(null); }, []);

  useEffect(() => {
    if (opened !== null && target === null) setOpened(null);
  }, [opened, target]);

  return <div className="overview-view">
    <div className="overview-view__feed" hidden={target !== null}>
      <OverviewPanel repos={config.repos} tabThemes={config.tabThemes} snapshot={target?.snapshot ?? snapshot}
        now={target?.now ?? now} documentHidden={feedHidden}
        onOpenRepo={onOpenRepo} onOpenFile={openFile} />
    </div>
    {target !== null && <OverviewWorkspace key={JSON.stringify([target.repo.repoId, target.repo.root, target.path])}
      repo={currentRepo ?? target.repo} path={target.path} isHandset={isHandset} onClose={closeFile} />}
  </div>;
}
