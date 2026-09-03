// Path: app/src/hooks/source_control/source_control_types.ts
// Description: State-machine and action contract exposed by useSourceControlState

import type {
  SourceControlActionKind,
  SourceControlScope,
  SourceControlStatus,
} from "../../shared/protocol.js";

/**
 * waiting -> loading -> ready | error; reconciling is entered when an action's outcome is
 * unknown (transport failure) and leaves on the next successful status read.
 */
export type SourceControlPhase =
  | { kind: "waiting" }
  | { kind: "loading" }
  | { kind: "ready" }
  | { kind: "error"; code: string | null; message: string }
  | { kind: "reconciling"; action: SourceControlActionKind };

/** A definitive rejection (GIT_* / INVALID_*) of one action; the git message is shown inline */
export interface SourceControlActionError {
  action: SourceControlActionKind;
  code: string;
  message: string;
}

/** The most recent commit this hook observed; `at` gives each commit a fresh identity */
export interface SourceControlCommit {
  sha: string;
  at: number;
}

export interface SourceControlState {
  phase: SourceControlPhase;
  /** Last known status for this repo; kept through loading/reconciling, cleared on hard errors */
  status: SourceControlStatus | null;
  /** index + worktree + conflicts; the SOURCE tab count */
  changeCount: number;
  /** Unmerged paths; non-zero is the exceptional merge state that outranks ordinary changes */
  conflictCount: number;
  /** One action at a time per repo; every action button is disabled while set */
  pendingAction: SourceControlActionKind | null;
  actionError: SourceControlActionError | null;
  lastCommit: SourceControlCommit | null;
  /** Draft commit message lives here so it survives rail switches; cleared on commit success */
  commitMessage: string;
  setCommitMessage: (message: string) => void;
  dismissActionError: () => void;
  refresh: () => void;
  stage: (scope: SourceControlScope) => void;
  unstage: (scope: SourceControlScope) => void;
  discard: (paths: string[]) => void;
  commit: (message: string) => void;
  push: () => void;
  pull: () => void;
}
