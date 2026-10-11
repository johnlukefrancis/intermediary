// Path: app/src/hooks/source_control/source_control_types.ts
// Description: State-machine and action contract exposed by useSourceControlState

import type {
  SourceControlActionKind,
  SourceControlDiscardTarget,
  SourceControlScope,
  SourceControlStatus,
} from "../../shared/protocol.js";
import type { SourceControlReadPhase } from "../../lib/source_control/status_registry_types.js";

/** Reconciling: entered when an action's outcome is unknown; left on a read with no mutation running or on budget expiry */
export type SourceControlPhase =
  | SourceControlReadPhase
  | { kind: "reconciling"; action: SourceControlActionKind };

/** A rejection the agent proved had no effect (`details.effect: notApplied`); shown inline */
export interface SourceControlActionError {
  action: SourceControlActionKind;
  code: string;
  message: string;
  /** The agent could not prove the action had no effect (a remote command): the text is Git's, the outcome is being reconciled */
  uncertain?: boolean;
}

/** The most recent commit this hook observed; `at` gives each commit a fresh identity */
export interface SourceControlCommit {
  sha: string;
  at: number;
}

/** The reviewed snapshot the COMMIT click freezes; a background refresh never rebinds it, and the
 * agent refuses when the repository has moved past it */
export interface SourceControlCommitRequest {
  message: string;
  /** `snapshotId` of the status that was on screen at the click; never empty */
  expectedSnapshotId: string;
}

export interface SourceControlState {
  phase: SourceControlPhase;
  /** Last known status for this repo; kept through loading/reconciling, cleared on hard errors */
  status: SourceControlStatus | null;
  /** Distinct changed files plus staged paths outside the root; the SOURCE tab count */
  changeCount: number;
  /** Unmerged paths; non-zero is the exceptional merge state that outranks ordinary changes */
  conflictCount: number;
  /** One action at a time per repo; every action button is disabled while set */
  pendingAction: SourceControlActionKind | null;
  actionError: SourceControlActionError | null;
  /** Paths a commit hook changed on the last successful commit; shown as a dismissible notice */
  hookNotice: string[] | null;
  /** Paths a commit hook added that no reviewed row covered; a dismissible warning-tone notice */
  hookAddedNotice: string[] | null;
  lastCommit: SourceControlCommit | null;
  /** Draft commit message lives here so it survives rail switches; cleared on commit success */
  commitMessage: string;
  setCommitMessage: (message: string) => void;
  dismissActionError: () => void;
  dismissHookNotice: () => void;
  dismissHookAddedNotice: () => void;
  refresh: () => void;
  stage: (scope: SourceControlScope) => void;
  unstage: (scope: SourceControlScope) => void;
  /** Every file the row owns, each with the stamp the UI reviewed when the file exists */
  discard: (targets: SourceControlDiscardTarget[]) => void;
  /** Sends exactly the frozen snapshot; never re-reads live status at send time */
  commit: (request: SourceControlCommitRequest) => void;
  push: () => void;
  pull: () => void;
}
