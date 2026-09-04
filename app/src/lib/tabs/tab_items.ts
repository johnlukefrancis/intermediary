// Path: app/src/lib/tabs/tab_items.ts
// Description: Tab-bar items derived from the configured repos: standalone tabs and grouped (worktree) tabs

import type { RepoConfig, RepoRoot } from "../../shared/config.js";

/** A standalone repo tab */
export interface SingleTab {
  type: "single";
  repoId: string;
  label: string;
  root: RepoRoot;
}

/** A grouped tab containing multiple repos with a dropdown */
export interface GroupTab {
  type: "group";
  groupId: string;
  groupLabel: string;
  repos: Array<{ repoId: string; label: string; root: RepoRoot }>;
}

export type TabItem = SingleTab | GroupTab;

/** Derive tabs from repos, grouping by groupId */
export function deriveTabsFromRepos(repos: RepoConfig[]): TabItem[] {
  const groupMap = new Map<string, GroupTab>();
  const tabs: TabItem[] = [];

  for (const repo of repos) {
    if (repo.groupId) {
      // Grouped repo - groupLabel is optional, fallback to groupId
      let group = groupMap.get(repo.groupId);
      if (!group) {
        group = {
          type: "group",
          groupId: repo.groupId,
          groupLabel: repo.groupLabel ?? repo.groupId,
          repos: [],
        };
        groupMap.set(repo.groupId, group);
        tabs.push(group);
      }
      // Update groupLabel if this repo has one and current label is the fallback
      if (repo.groupLabel && group.groupLabel === group.groupId) {
        group.groupLabel = repo.groupLabel;
      }
      group.repos.push({ repoId: repo.repoId, label: repo.label, root: repo.root });
    } else {
      // Standalone repo
      tabs.push({
        type: "single",
        repoId: repo.repoId,
        label: repo.label,
        root: repo.root,
      });
    }
  }

  return tabs;
}
