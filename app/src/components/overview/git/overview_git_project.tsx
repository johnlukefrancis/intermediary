// Path: app/src/components/overview/git/overview_git_project.tsx
// Description: One grouped project in the Git column: the group's accent rule over a block per worktree with changes

import type React from "react";
import { useMemo } from "react";
import type { RepoConfig } from "../../../shared/config.js";
import type { SourceControlEntry } from "../../../shared/protocol.js";
import type { OverviewSnapshot } from "../../../lib/overview/overview_types.js";
import { hexToAccentCssVars } from "../../../lib/theme/accent_utils.js";
import { OverviewGitRepo } from "./overview_git_repo.js";
import type { Lane } from "../../../lib/overview/overview_lanes.js";
import type { GitLaneState } from "../../../hooks/overview/use_overview_git_states.js";

interface OverviewGitProjectProps {
  label: string;
  accentHex: string;
  members: readonly RepoConfig[];
  snapshot: OverviewSnapshot;
  lanes: ReadonlyMap<string, Lane>;
  states: ReadonlyMap<string, GitLaneState>;
  onOpenRepoSource: (repoId: string) => void;
  onOpenDiff: (repo: RepoConfig, entry: SourceControlEntry) => void;
}

export function OverviewGitProject({ label, accentHex, members, snapshot, lanes, states, onOpenRepoSource, onOpenDiff }: OverviewGitProjectProps): React.JSX.Element {
  const style = useMemo(() => hexToAccentCssVars(accentHex) as React.CSSProperties, [accentHex]);
  return (
    <section className="overview-git-project" style={style} aria-label={`${label} worktrees`}>
      <header className="overview-git-project__head">
        <span className="overview-git-repo__dot" aria-hidden="true" />
        <strong>{label}</strong>
        <span className="overview-git-repo__rule" aria-hidden="true" />
      </header>
      {members.map((repo) => <OverviewGitRepo key={repo.repoId} repo={repo} accentHex={accentHex} worktree
        availability={snapshot.repos.get(repo.repoId)?.availability ?? "connecting"}
        glow={states.get(repo.repoId)?.glow ?? null} finishedAt={lanes.get(repo.repoId)?.finishedAt ?? null}
        onOpenRepoSource={onOpenRepoSource} onOpenDiff={onOpenDiff} />)}
    </section>
  );
}
