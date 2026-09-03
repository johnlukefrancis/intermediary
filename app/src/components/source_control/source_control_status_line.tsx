// Path: app/src/components/source_control/source_control_status_line.tsx
// Description: Branch, ahead/behind, HEAD sha, and refresh/pull/push controls for the Source Control column

import type React from "react";
import type { SourceControlStatus } from "../../shared/protocol.js";
import type {
  SourceControlCommit,
  SourceControlPhase,
} from "../../hooks/source_control/source_control_types.js";
import { aheadBehindLabel, branchLabel, shortSha } from "./source_control_copy.js";
import { PullIcon, PushIcon, RefreshIcon } from "./source_control_icons.js";

interface SourceControlStatusLineProps {
  status: SourceControlStatus;
  phase: SourceControlPhase;
  lastCommit: SourceControlCommit | null;
  refreshDisabled: boolean;
  /** Pull/push: disabled while any action is pending or status is not usable */
  remoteDisabled: boolean;
  onRefresh: () => void;
  onPull: () => void;
  onPush: () => void;
}

export function SourceControlStatusLine({
  status,
  phase,
  lastCommit,
  refreshDisabled,
  remoteDisabled,
  onRefresh,
  onPull,
  onPush,
}: SourceControlStatusLineProps): React.JSX.Element {
  const branch = branchLabel(status);
  const aheadBehind = aheadBehindLabel(status);
  const sha = status.detached ? null : shortSha(status.headSha);
  const isLoading = phase.kind === "loading";
  const isFresh = lastCommit !== null && status.headSha === lastCommit.sha;
  const pullTitle =
    status.upstream === null
      ? "No upstream configured; nothing to pull"
      : `Pull from ${status.upstream} (fast-forward only)`;
  const pushTitle =
    status.upstream === null
      ? "Push and set upstream (needs exactly one remote)"
      : `Push to ${status.upstream}`;

  return (
    <div className="source-control-status" aria-busy={isLoading}>
      <div className="source-control-status__branch">
        {!status.detached && <span className="source-control-status__glyph" aria-hidden="true">⎇</span>}
        <span
          className="source-control-status__name"
          data-detached={status.detached || undefined}
          title={status.detached ? "Detached HEAD" : `On branch ${branch}`}
        >
          {branch}
        </span>
        {aheadBehind !== null && (
          <span className="source-control-status__ab" title={`Upstream ${status.upstream ?? ""}`}>
            {aheadBehind}
          </span>
        )}
        {sha !== null && (
          <span
            key={isFresh ? lastCommit.at : "head"}
            className="source-control-status__sha"
            data-fresh={isFresh || undefined}
            title={status.headSha ?? undefined}
          >
            {sha}
          </span>
        )}
      </div>
      <div className="source-control-status__actions">
        <button
          type="button"
          className="panel-header-icon source-control-status__button"
          data-busy={isLoading || undefined}
          disabled={refreshDisabled}
          onClick={onRefresh}
          title="Refresh status"
          aria-label="Refresh status"
        >
          <RefreshIcon />
        </button>
        <button
          type="button"
          className="panel-header-icon source-control-status__button"
          disabled={remoteDisabled || status.upstream === null}
          onClick={onPull}
          title={pullTitle}
          aria-label="Pull"
        >
          <PullIcon />
        </button>
        <button
          type="button"
          className="panel-header-icon source-control-status__button"
          disabled={remoteDisabled}
          onClick={onPush}
          title={pushTitle}
          aria-label="Push"
        >
          <PushIcon />
        </button>
      </div>
    </div>
  );
}
