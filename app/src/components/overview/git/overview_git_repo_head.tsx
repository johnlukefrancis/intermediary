// Path: app/src/components/overview/git/overview_git_repo_head.tsx
// Description: Git column repository head: accent label to Stream, reread pulse, SOURCE link, branch line and state badges

import type React from "react";
import type { RepoConfig } from "../../../shared/config.js";
import type { SourceControlStatus } from "../../../shared/protocol.js";
import { totalConflictCount } from "../../../lib/source_control/conflict_count.js";
import { aheadBehindLabel, branchLabel } from "../../source_control/source_control_copy.js";

interface OverviewGitRepoHeadProps {
  repo: RepoConfig;
  status: SourceControlStatus | null;
  /** A reread is in flight behind the kept status */
  rereading: boolean;
  onOpenSource: () => void;
}

function BranchLine({ status }: { status: SourceControlStatus }): React.JSX.Element {
  const aheadBehind = aheadBehindLabel(status);
  const conflicts = totalConflictCount(status);
  return (
    <div className="overview-git-repo__meta">
      {!status.detached && <span className="overview-git-repo__glyph" aria-hidden="true">⎇</span>}
      <span className="overview-git-repo__branch" data-detached={status.detached || undefined}
        title={status.detached ? `Detached HEAD ${status.headSha ?? ""}` : `On branch ${branchLabel(status)}`}>{branchLabel(status)}</span>
      {aheadBehind !== null && <span className="overview-git-repo__ab" title={`Upstream ${status.upstream ?? ""}`}>{aheadBehind}</span>}
      {conflicts > 0 && <span className="badge badge--error">{conflicts === 1 ? "1 CONFLICT" : `${String(conflicts)} CONFLICTS`}</span>}
      {status.truncated && <span className="badge badge--warning" title="Git status overran its budget; lists are incomplete">TRUNCATED</span>}
      {status.mutationInProgress && <span className="badge badge--warning" title="A source-control action held this repository while it was read">MID-OPERATION</span>}
    </div>
  );
}

export function OverviewGitRepoHead({ repo, status, rereading, onOpenSource }: OverviewGitRepoHeadProps): React.JSX.Element {
  return (
    <header className="overview-git-repo__head">
      <div className="overview-git-repo__title">
        <button type="button" className="overview-git-repo__link" onClick={onOpenSource}
          title={`Open ${repo.label} Source Control`} aria-label={`Open ${repo.label} Source Control`}>
          <span className="overview-git-repo__dot" aria-hidden="true" />
          <strong>{repo.label}</strong>
        </button>
        <span className="overview-git-repo__rule" aria-hidden="true" />
        {rereading && <span className="overview-git-repo__pulse" role="status" aria-label="Reading working tree" />}
        <button type="button" className="overview-git-repo__source" onClick={onOpenSource}
          title={`Open ${repo.label} Source Control`} aria-label={`Open ${repo.label} Source Control`}>
          <span>SOURCE</span>
          <svg className="overview-git-repo__chevron" viewBox="0 0 24 24" aria-hidden="true"><path d="M9 6l6 6-6 6" /></svg>
        </button>
      </div>
      {status !== null && <BranchLine status={status} />}
    </header>
  );
}
