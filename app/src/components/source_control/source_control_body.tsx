// Path: app/src/components/source_control/source_control_body.tsx
// Description: Phase-dependent body of the Source Control column: empty states or the three sections

import type React from "react";
import type { SourceControlEntry, SourceControlStatus } from "../../shared/protocol.js";
import type { SourceControlPhase } from "../../hooks/source_control/source_control_types.js";
import {
  MERGE_CONFLICTS_TITLE,
  NO_CHANGES,
  READING_WORKING_TREE,
  reconcilingCopy,
  statusErrorCopy,
} from "./source_control_copy.js";
import { SourceControlSection } from "./source_control_section.js";

interface SourceControlBodyProps {
  status: SourceControlStatus | null;
  phase: SourceControlPhase;
  /** Per-row and bulk actions are disabled while an action is pending or status is not ready */
  actionsDisabled: boolean;
  onRefresh: () => void;
  onStageAll: () => void;
  onUnstageAll: () => void;
  onStageEntry: (entry: SourceControlEntry) => void;
  onUnstageEntry: (entry: SourceControlEntry) => void;
  onOpenDiff: (entry: SourceControlEntry) => void;
  onContextMenu: (event: React.MouseEvent, entry: SourceControlEntry) => void;
}

export function SourceControlBody({
  status,
  phase,
  actionsDisabled,
  onRefresh,
  onStageAll,
  onUnstageAll,
  onStageEntry,
  onUnstageEntry,
  onOpenDiff,
  onContextMenu,
}: SourceControlBodyProps): React.JSX.Element {
  if (status === null) {
    if (phase.kind === "error") {
      const copy = statusErrorCopy(phase.code);
      return (
        <div className="source-control-empty">
          <p className="empty-state">{copy.heading}</p>
          {copy.showMessage && phase.message.length > 0 && (
            <div className="build-error source-control-notice">{phase.message}</div>
          )}
          <button type="button" className="dir-action-btn" onClick={onRefresh}>
            Refresh
          </button>
        </div>
      );
    }
    return (
      <p className="empty-state empty-state--waiting">
        {phase.kind === "reconciling" ? reconcilingCopy(phase.action) : READING_WORKING_TREE}
      </p>
    );
  }

  const changeCount = status.index.length + status.worktree.length + status.conflicts.length;
  const rowProps = { disabled: actionsDisabled, onOpenDiff, onContextMenu };

  return (
    <>
      {phase.kind === "reconciling" && (
        <p className="empty-state empty-state--waiting source-control-banner" role="status">
          {reconcilingCopy(phase.action)}
        </p>
      )}
      {changeCount === 0 ? (
        <p className="empty-state">{NO_CHANGES}</p>
      ) : (
        <div className="source-control-sections">
          {status.conflicts.length > 0 && (
            <SourceControlSection
              title={MERGE_CONFLICTS_TITLE}
              tone="alert"
              entries={status.conflicts}
              rowAction="stage"
              onRowAction={onStageEntry}
              {...rowProps}
            />
          )}
          <SourceControlSection
            title="STAGED CHANGES"
            entries={status.index}
            rowAction="unstage"
            bulk={{
              kind: "unstage",
              title: "Unstage all changes",
              disabled: actionsDisabled || status.index.length === 0,
              onClick: onUnstageAll,
            }}
            onRowAction={onUnstageEntry}
            {...rowProps}
          />
          <SourceControlSection
            title="CHANGES"
            entries={status.worktree}
            rowAction="stage"
            bulk={{
              kind: "stage",
              title: status.truncated
                ? "Stage all changes (disabled while status is truncated)"
                : "Stage all changes",
              disabled: actionsDisabled || status.truncated || status.worktree.length === 0,
              onClick: onStageAll,
            }}
            onRowAction={onStageEntry}
            {...rowProps}
          />
        </div>
      )}
    </>
  );
}
