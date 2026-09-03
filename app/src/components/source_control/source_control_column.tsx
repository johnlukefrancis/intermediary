// Path: app/src/components/source_control/source_control_column.tsx
// Description: Source Control column frame: status line, warnings, commit box, notices, body, menus, confirms

import type React from "react";
import { useCallback, useState } from "react";
import type { SourceControlEntry } from "../../shared/protocol.js";
import type { SourceControlState } from "../../hooks/source_control/source_control_types.js";
import { useConfig } from "../../hooks/use_config.js";
import { useFileActions } from "../../hooks/use_file_actions.js";
import { ConfirmModal } from "../confirm_modal.js";
import { ContextMenu } from "../context_menu.js";
import { SourceControlBody } from "./source_control_body.js";
import { SourceControlCommitBox } from "./source_control_commit_box.js";
import { buildSourceControlContextMenuItems } from "./source_control_context_menu.js";
import {
  STAGE_TO_COMMIT_HINT,
  TRUNCATED_HINT,
  actionErrorHeading,
  branchLabel,
} from "./source_control_copy.js";
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

/** Renamed/copied entries carry both sides so stage/unstage moves the whole rename */
function entryPaths(entry: SourceControlEntry): string[] {
  return entry.originalPath !== undefined ? [entry.originalPath, entry.path] : [entry.path];
}

function discardMessage(entry: SourceControlEntry): string {
  return entry.change === "untracked"
    ? `Delete untracked file "${entry.path}"? This cannot be undone.`
    : `Discard changes to "${entry.path}"? The working tree copy is replaced from the index and cannot be recovered.`;
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
  const [commitConfirmOpen, setCommitConfirmOpen] = useState(false);
  const repoRoot = config.repos.find((repo) => repo.repoId === repoId)?.root;
  const {
    status, phase, pendingAction, actionError, lastCommit, commitMessage,
    setCommitMessage, dismissActionError, refresh, stage, unstage, discard, commit, push, pull,
  } = state;
  // A background refetch (loading with a status in hand) never disables the controls;
  // the action result supersedes any in-flight read.
  const isReady = phase.kind === "ready" || (phase.kind === "loading" && status !== null);
  const actionsDisabled = pendingAction !== null || !isReady;

  const stageEntry = useCallback((entry: SourceControlEntry) => {
    stage({ mode: "paths", paths: entryPaths(entry) });
  }, [stage]);
  const unstageEntry = useCallback((entry: SourceControlEntry) => {
    unstage({ mode: "paths", paths: entryPaths(entry) });
  }, [unstage]);
  const stageAll = useCallback(() => { stage({ mode: "all" }); }, [stage]);
  const unstageAll = useCallback(() => { unstage({ mode: "all" }); }, [unstage]);
  const openContextMenu = useCallback((event: React.MouseEvent, entry: SourceControlEntry) => {
    setContextMenu({ x: event.clientX, y: event.clientY, entry });
  }, []);
  const closeContextMenu = useCallback(() => { setContextMenu(null); }, []);

  // Git's own committability (index differs from HEAD, or a merge is in progress) decides,
  // not the root-projected list: a merge resolved to HEAD's tree still needs its commit.
  const hint =
    status === null ? null
      : !status.committable ? STAGE_TO_COMMIT_HINT
        : status.truncated ? TRUNCATED_HINT
          : null;
  const canCommit =
    status !== null && !actionsDisabled && hint === null && commitMessage.trim().length > 0;

  const requestCommit = useCallback(() => {
    if (!canCommit) return;
    if (status.omitted.stagedOutsideRoot > 0) {
      setCommitConfirmOpen(true);
      return;
    }
    commit(commitMessage);
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
            {actionErrorHeading(actionError.action)}
          </span>
          <span className="source-control-notice__message">{actionError.message}</span>
          <button type="button" className="dir-action-btn" onClick={dismissActionError}>
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
          message={discardMessage(discardTarget)}
          confirmLabel="Discard"
          isDestructive
          onConfirm={() => {
            discard(entryPaths(discardTarget));
            setDiscardTarget(null);
          }}
          onCancel={() => { setDiscardTarget(null); }}
        />
      )}
      {commitConfirmOpen && status !== null && (
        <ConfirmModal
          title="Commit staged changes"
          message={`${status.omitted.stagedOutsideRoot} staged path(s) outside this folder will also be committed. Continue?`}
          confirmLabel="Commit"
          onConfirm={() => {
            setCommitConfirmOpen(false);
            commit(commitMessage);
          }}
          onCancel={() => { setCommitConfirmOpen(false); }}
        />
      )}
    </div>
  );
}
