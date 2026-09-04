// Path: app/src/components/source_control/source_control_column.tsx
// Description: Source Control column frame: status line, warnings, commit box, notices, body, menus, confirms

import type React from "react";
import { useCallback, useState } from "react";
import type { SourceControlDiscardTarget, SourceControlEntry } from "../../shared/protocol.js";
import type {
  SourceControlCommitRequest,
  SourceControlState,
} from "../../hooks/source_control/source_control_types.js";
import { useConfig } from "../../hooks/use_config.js";
import { useFileActions } from "../../hooks/use_file_actions.js";
import { ConfirmModal } from "../confirm_modal.js";
import { ContextMenu } from "../context_menu.js";
import { SourceControlBody } from "./source_control_body.js";
import { SourceControlCommitBox } from "./source_control_commit_box.js";
import { buildSourceControlContextMenuItems } from "./source_control_context_menu.js";
import {
  HOOK_ADDED_HEADING,
  NO_SNAPSHOT_HINT,
  STAGE_TO_COMMIT_HINT,
  TRUNCATED_HINT,
  actionErrorHeading,
  branchLabel,
  discardConfirmMessage,
  hookAddedMessage,
  hookChangedHeading,
  hookChangedMessage,
  resolveConflictsHint,
} from "./source_control_copy.js";
import { totalConflictCount } from "../../lib/source_control/conflict_count.js";
import { SourceControlStatusLine } from "./source_control_status_line.js";
import { SourceControlWarnings } from "./source_control_warnings.js";

interface SourceControlColumnProps {
  repoId: string;
  state: SourceControlState;
  onOpenDiff: (entry: SourceControlEntry) => void;
}

interface ContextMenuState {
  x: number;
  y: number;
  entry: SourceControlEntry;
}

/**
 * The files a row owns. A rename is one change with two endpoints, so both travel together; a
 * copy's `originalPath` is only provenance — acting on it would touch an unrelated file.
 */
function rowTargets(entry: SourceControlEntry): string[] {
  return entry.change === "renamed" && entry.originalPath !== undefined
    ? [entry.originalPath, entry.path]
    : [entry.path];
}

/** The stamp the UI reviewed travels with the target it belongs to, so a stale file is refused */
function discardTargets(entry: SourceControlEntry): SourceControlDiscardTarget[] {
  return rowTargets(entry).map((path) => {
    if (path !== entry.path) return { path };
    if (entry.worktreeMissing === true) return { path, expectedMissing: true as const };
    if (entry.worktreeStamp !== undefined) return { path, expectedStamp: entry.worktreeStamp };
    return { path };
  });
}

/**
 * The exact status the COMMIT click reviewed, frozen at that moment. When outside-root paths
 * require confirmation this is what the modal is built from and what it re-renders from on every
 * background status refresh while it stays open — never the live status.
 */
interface PendingCommitRequest extends SourceControlCommitRequest {
  stagedOutsideRootCount: number;
}

export function SourceControlColumn({
  repoId,
  state,
  onOpenDiff,
}: SourceControlColumnProps): React.JSX.Element {
  const { config } = useConfig();
  const fileActions = useFileActions();
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  const [discardTarget, setDiscardTarget] = useState<SourceControlEntry | null>(null);
  const [pendingCommit, setPendingCommit] = useState<PendingCommitRequest | null>(null);
  const repoRoot = config.repos.find((repo) => repo.repoId === repoId)?.root;
  const {
    status, phase, pendingAction, actionError, hookNotice, hookAddedNotice, lastCommit,
    commitMessage, setCommitMessage, dismissActionError, dismissHookNotice,
    dismissHookAddedNotice, refresh, stage, unstage, discard, commit, push, pull,
  } = state;
  // A background refetch (loading with a status in hand) never disables the controls;
  // the action result supersedes any in-flight read.
  const isReady = phase.kind === "ready" || (phase.kind === "loading" && status !== null);
  const actionsDisabled = pendingAction !== null || !isReady;

  const stageEntry = useCallback((entry: SourceControlEntry) => {
    stage({ mode: "paths", paths: rowTargets(entry) });
  }, [stage]);
  const unstageEntry = useCallback((entry: SourceControlEntry) => {
    unstage({ mode: "paths", paths: rowTargets(entry) });
  }, [unstage]);
  const stageAll = useCallback(() => { stage({ mode: "all" }); }, [stage]);
  const unstageAll = useCallback(() => { unstage({ mode: "all" }); }, [unstage]);
  const openContextMenu = useCallback((event: React.MouseEvent, entry: SourceControlEntry) => {
    setContextMenu({ x: event.clientX, y: event.clientY, entry });
  }, []);
  const closeContextMenu = useCallback(() => { setContextMenu(null); }, []);

  // Git's own committability (index differs from HEAD, or a merge is in progress) decides,
  // not the root-projected list: a merge resolved to HEAD's tree still needs its commit.
  // Unmerged paths make Git refuse the commit outright, so they outrank every other hint.
  // An empty snapshotId is the torn review: there is no state the agent could check a commit
  // against, so it is the last blocker in the chain rather than a banner of its own.
  const conflictCount = status === null ? 0 : totalConflictCount(status);
  const hint =
    status === null ? null
      : conflictCount > 0 ? resolveConflictsHint(conflictCount)
        : !status.committable ? STAGE_TO_COMMIT_HINT
          : status.truncated ? TRUNCATED_HINT
            : status.snapshotId.length === 0 ? NO_SNAPSHOT_HINT
              : null;
  const canCommit =
    status !== null && !actionsDisabled && hint === null && commitMessage.trim().length > 0;

  // Freezes the reviewed snapshot once, at the click, and never rebinds it to a later refresh:
  // both the wire request and the outside-root modal's own re-renders read only this object.
  const requestCommit = useCallback(() => {
    if (!canCommit) return;
    const request: PendingCommitRequest = {
      message: commitMessage,
      expectedSnapshotId: status.snapshotId,
      stagedOutsideRootCount: status.omitted.stagedOutsideRoot,
    };
    if (request.stagedOutsideRootCount > 0) {
      setPendingCommit(request);
      return;
    }
    commit(request);
  }, [canCommit, commit, commitMessage, status]);

  const contextMenuItems = contextMenu && repoRoot
    ? buildSourceControlContextMenuItems({
      entry: contextMenu.entry,
      repoRoot,
      fileActions,
      actionsDisabled,
      onStage: stageEntry,
      onUnstage: unstageEntry,
      onOpenDiff,
      onDiscard: setDiscardTarget,
    })
    : [];

  return (
    <div className="source-control" data-phase={phase.kind}>
      {status !== null && (
        <>
          <SourceControlStatusLine
            status={status}
            phase={phase}
            lastCommit={lastCommit}
            refreshDisabled={pendingAction !== null || phase.kind === "waiting"}
            remoteDisabled={actionsDisabled}
            onRefresh={refresh}
            onPull={pull}
            onPush={push}
          />
          <SourceControlWarnings status={status} />
          <SourceControlCommitBox
            message={commitMessage}
            branch={branchLabel(status)}
            canCommit={canCommit}
            isCommitting={pendingAction === "commit"}
            disabled={pendingAction !== null}
            hint={hint}
            onMessageChange={setCommitMessage}
            onCommit={requestCommit}
          />
        </>
      )}
      {actionError && (
        <div className="build-error source-control-notice" role="alert">
          <span className="source-control-notice__heading">
            {actionErrorHeading(actionError.action, actionError.code, actionError.uncertain)}
          </span>
          <span className="source-control-notice__message">{actionError.message}</span>
          <button type="button" className="dir-action-btn" onClick={dismissActionError}>
            Dismiss
          </button>
        </div>
      )}
      {hookNotice && (
        <div className="source-control-notice source-control-notice--info" role="status">
          <span className="source-control-notice__heading">
            {hookChangedHeading(hookNotice.length)}
          </span>
          <span className="source-control-notice__message">{hookChangedMessage(hookNotice)}</span>
          <button type="button" className="dir-action-btn" onClick={dismissHookNotice}>
            Dismiss
          </button>
        </div>
      )}
      {hookAddedNotice && (
        <div className="source-control-notice source-control-notice--warning" role="alert">
          <span className="source-control-notice__heading">{HOOK_ADDED_HEADING}</span>
          <span className="source-control-notice__message">
            {hookAddedMessage(hookAddedNotice)}
          </span>
          <button type="button" className="dir-action-btn" onClick={dismissHookAddedNotice}>
            Dismiss
          </button>
        </div>
      )}
      <SourceControlBody
        status={status}
        phase={phase}
        actionsDisabled={actionsDisabled}
        onRefresh={refresh}
        onStageAll={stageAll}
        onUnstageAll={unstageAll}
        onStageEntry={stageEntry}
        onUnstageEntry={unstageEntry}
        onOpenDiff={onOpenDiff}
        onContextMenu={openContextMenu}
      />
      {contextMenu && repoRoot && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          items={contextMenuItems}
          onClose={closeContextMenu}
        />
      )}
      {discardTarget && (
        <ConfirmModal
          title="Discard changes"
          message={discardConfirmMessage(discardTarget, rowTargets(discardTarget))}
          confirmLabel="Discard"
          isDestructive
          onConfirm={() => {
            discard(discardTargets(discardTarget));
            setDiscardTarget(null);
          }}
          onCancel={() => { setDiscardTarget(null); }}
        />
      )}
      {pendingCommit && (
        <ConfirmModal
          title="Commit staged changes"
          message={`${pendingCommit.stagedOutsideRootCount} staged path(s) outside this folder will also be committed. Continue?`}
          confirmLabel="Commit"
          onConfirm={() => {
            commit(pendingCommit);
            setPendingCommit(null);
          }}
          onCancel={() => { setPendingCommit(null); }}
        />
      )}
    </div>
  );
}
