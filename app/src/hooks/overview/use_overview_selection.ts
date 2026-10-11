// Path: app/src/hooks/overview/use_overview_selection.ts
// Description: Overview's one repository filter: working set, chip selection and the shown repositories for both columns

import { useMemo, useState } from "react";
import type { RepoConfig } from "../../shared/config.js";
import type { OverviewSnapshot } from "../../lib/overview/overview_types.js";
import { isWorking } from "../../lib/overview/overview_focus.js";

export interface OverviewSelection {
  /** Recently worked repositories in config order; identity changes only when membership does */
  working: readonly RepoConfig[];
  selection: string | null;
  shown: readonly RepoConfig[];
  select: (repoId: string | null) => void;
}

export function useOverviewSelection(repos: readonly RepoConfig[], snapshot: OverviewSnapshot, now: number): OverviewSelection {
  const [selected, setSelected] = useState<string | null>(null);
  const workingKey = repos.filter((repo) => isWorking(snapshot.repos.get(repo.repoId), now)).map((repo) => repo.repoId).join("\0");
  const working = useMemo(() => {
    const ids = new Set(workingKey.split("\0"));
    return repos.filter((repo) => ids.has(repo.repoId));
  }, [repos, workingKey]);
  const selection = working.some((repo) => repo.repoId === selected) ? selected : null;
  const shown = useMemo(() => selection === null ? working : working.filter((repo) => repo.repoId === selection), [selection, working]);
  return { working, selection, shown, select: setSelected };
}
