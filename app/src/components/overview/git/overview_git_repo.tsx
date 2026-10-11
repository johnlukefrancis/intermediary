// Path: app/src/components/overview/git/overview_git_repo.tsx
// Description: One Git column working-tree block (a repository, or one worktree of a project): status, state lines, groups

import type React from "react";
import { memo, useCallback, useMemo } from "react";
import type { RepoConfig } from "../../../shared/config.js";
import type { SourceControlEntry, SourceControlStatus } from "../../../shared/protocol.js";
import type { OverviewAvailability } from "../../../lib/overview/overview_types.js";
import type { SourceControlReadPhase } from "../../../lib/source_control/status_registry_types.js";
import { statusGroups } from "../../../lib/source_control/status_groups.js";
import { hexToAccentCssVars } from "../../../lib/theme/accent_utils.js";
import { useSourceControlStatus } from "../../../hooks/source_control/use_source_control_status.js";
import { READING_WORKING_TREE, statusErrorCopy } from "../../source_control/source_control_copy.js";
import { OverviewGitRepoHead } from "./overview_git_repo_head.js";
import { OverviewGitGroup } from "./overview_git_group.js";

/** Watch facts that mean no status read can answer; the registry skips these roots itself */
const HELD_COPY: Readonly<Partial<Record<OverviewAvailability, string>>> = {
  held: "WSL HELD", "update-required": "UPDATE REQUIRED", offline: "OFFLINE",
};

interface OverviewGitRepoProps {
  repo: RepoConfig;
  accentHex: string;
  /** Nested under its project's rule as one worktree */
  worktree: boolean;
  availability: OverviewAvailability;
  onOpenRepoSource: (repoId: string) => void;
  onOpenDiff: (repo: RepoConfig, entry: SourceControlEntry) => void;
}

function StateLine({ tone, children, title }: { tone: string; children: string; title?: string | undefined }): React.JSX.Element {
  return <p className="overview-git-repo__state" data-tone={tone} title={title}>{children}</p>;
}

function Body({ status, phase, held, onOpenDiff, onOpenSource }: {
  status: SourceControlStatus | null;
  phase: SourceControlReadPhase;
  held: string | undefined;
  onOpenDiff: (entry: SourceControlEntry) => void;
  onOpenSource: () => void;
}): React.JSX.Element {
  const groups = useMemo(() => status === null ? [] : statusGroups(status), [status]);
  if (held !== undefined) return <StateLine tone="held">{held}</StateLine>;
  if (status === null) {
    if (phase.kind !== "error") return <StateLine tone="waiting">{READING_WORKING_TREE}</StateLine>;
    return <StateLine tone="error" title={phase.message || undefined}>{statusErrorCopy(phase.code).heading}</StateLine>;
  }
  return <>{groups.map((group) => <OverviewGitGroup key={group.kind} kind={group.kind} entries={group.entries}
    onOpenDiff={onOpenDiff} onOpenSource={onOpenSource} />)}</>;
}

export const OverviewGitRepo = memo(function OverviewGitRepo({
  repo, accentHex, worktree, availability, onOpenRepoSource, onOpenDiff,
}: OverviewGitRepoProps): React.JSX.Element {
  const { status, phase } = useSourceControlStatus(repo.repoId);
  const style = useMemo(() => hexToAccentCssVars(accentHex) as React.CSSProperties, [accentHex]);
  const openSource = useCallback(() => { onOpenRepoSource(repo.repoId); }, [onOpenRepoSource, repo.repoId]);
  const openDiff = useCallback((entry: SourceControlEntry) => { onOpenDiff(repo, entry); }, [onOpenDiff, repo]);
  const held = HELD_COPY[availability];
  return (
    <section className="overview-git-repo" style={style} data-worktree={worktree || undefined} data-repo-id={repo.repoId} aria-label={`${repo.label} working tree`}>
      <OverviewGitRepoHead repo={repo} status={status} rereading={held === undefined && status !== null && phase.kind === "loading"}
        onOpenSource={openSource} />
      <Body status={status} phase={phase} held={held} onOpenDiff={openDiff} onOpenSource={openSource} />
    </section>
  );
});
