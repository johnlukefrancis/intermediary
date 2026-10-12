// Path: app/src/components/overview/git/overview_git_column.tsx
// Description: Overview's read-only Git column: changed working trees, worktrees nested under their project group

import type React from "react";
import { useEffect, useMemo, useRef } from "react";
import type { RepoConfig, TabTheme } from "../../../shared/config.js";
import type { SourceControlEntry } from "../../../shared/protocol.js";
import type { OverviewSnapshot } from "../../../lib/overview/overview_types.js";
import { statusRegistry } from "../../../lib/source_control/status_registry.js";
import { DEFAULT_ACCENT_HEX } from "../../../lib/theme/accent_utils.js";
import { useOverviewGitStates } from "../../../hooks/overview/use_overview_git_states.js";
import type { Lane } from "../../../lib/overview/overview_lanes.js";
import { OverviewGitRepo } from "./overview_git_repo.js";
import { OverviewGitProject } from "./overview_git_project.js";
import "../../../styles/overview/overview_git.css";

interface OverviewGitColumnProps {
  working: readonly RepoConfig[];
  shown: readonly RepoConfig[];
  tabThemes: Readonly<Record<string, TabTheme>>;
  snapshot: OverviewSnapshot;
  lanes: ReadonlyMap<string, Lane>;
  now: number;
  onOpenRepoSource: (repoId: string) => void;
  onOpenDiff: (repo: RepoConfig, entry: SourceControlEntry) => void;
}

interface GitBlock { key: string; label: string; grouped: boolean; members: RepoConfig[] }

/** Config groups are worktrees of one project; first appearance keeps the shown order */
function gitBlocks(repos: readonly RepoConfig[]): GitBlock[] {
  const blocks = new Map<string, GitBlock>();
  for (const repo of repos) {
    const key = repo.groupId ?? repo.repoId;
    const block = blocks.get(key);
    if (block !== undefined) block.members.push(repo);
    else blocks.set(key, { key, label: repo.groupLabel ?? repo.groupId ?? repo.label, grouped: repo.groupId !== undefined, members: [repo] });
  }
  return [...blocks.values()];
}

/** Holds demand on every working repository so switching chips never starts a read */
function useRetainedRepos(working: readonly RepoConfig[]): void {
  const held = useRef(new Map<string, () => void>());
  const workingKey = working.map((repo) => repo.repoId).join("\0");
  useEffect(() => {
    const map = held.current;
    const next = new Set(workingKey.split("\0").filter((repoId) => repoId.length > 0));
    for (const [repoId, release] of map) {
      if (next.has(repoId)) continue;
      release();
      map.delete(repoId);
    }
    for (const repoId of next) if (!map.has(repoId)) map.set(repoId, statusRegistry.retain(repoId));
  }, [workingKey]);
  useEffect(() => {
    const map = held.current;
    return () => {
      for (const release of map.values()) release();
      map.clear();
    };
  }, []);
}

export function OverviewGitColumn({ working, shown, tabThemes, snapshot, lanes, now, onOpenRepoSource, onOpenDiff }: OverviewGitColumnProps): React.JSX.Element {
  useRetainedRepos(working);
  const states = useOverviewGitStates(shown, lanes, snapshot, now);
  const visible = useMemo(() => shown.filter(repo => states.get(repo.repoId)?.dirty || states.get(repo.repoId)?.glow === "finished"), [states, shown]);
  const changed = shown.filter(repo => states.get(repo.repoId)?.dirty).length;
  const blocks = useMemo(() => gitBlocks(visible), [visible]);
  const accentOf = (key: string): string => tabThemes[key]?.accentHex ?? DEFAULT_ACCENT_HEX;
  return (
    <section className="panel overview-git" aria-label="Working tree">
      <header className="overview-git__header">
        <h2 className="panel-title">WORKING TREE</h2>
        <span className="overview-git__count">{changed === 1 ? "1 CHANGED" : `${String(changed)} CHANGED`}</span>
      </header>
      <div className="overview-git__shell">
        <div className="overview-git__scroller">
          {blocks.length === 0 && <p className="overview-git-repo__state" data-tone="clean">ALL CLEAN</p>}
          {blocks.map((block) => block.grouped
            ? <OverviewGitProject key={block.key} label={block.label} accentHex={accentOf(block.key)} members={block.members}
              snapshot={snapshot} lanes={lanes} states={states} onOpenRepoSource={onOpenRepoSource} onOpenDiff={onOpenDiff} />
            : block.members.map((repo) => <OverviewGitRepo key={repo.repoId} repo={repo} accentHex={accentOf(block.key)} worktree={false}
              availability={snapshot.repos.get(repo.repoId)?.availability ?? "connecting"}
              glow={states.get(repo.repoId)?.glow ?? null} finishedAt={lanes.get(repo.repoId)?.finishedAt ?? null}
              onOpenRepoSource={onOpenRepoSource} onOpenDiff={onOpenDiff} />))}
        </div>
      </div>
    </section>
  );
}
