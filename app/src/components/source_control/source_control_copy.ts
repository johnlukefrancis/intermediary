// Path: app/src/components/source_control/source_control_copy.ts
// Description: Badge letters, branch labels, and empty-state/error copy for the Source Control column

import {
  AGENT_DRAINING_CODE,
  SOURCE_CONTROL_STATE_CHANGED_CODE,
  type SourceControlActionKind,
  type SourceControlChange,
  type SourceControlEntry,
  type SourceControlStatus,
} from "../../shared/protocol.js";

export interface ChangeBadge {
  letter: string;
  /** Suffix of the `.badge--*` class; never `staged`, which means drag-handoff staging */
  variant: string;
  label: string;
}

export const CHANGE_BADGES: Record<SourceControlChange, ChangeBadge> = {
  added: { letter: "A", variant: "add", label: "Added" },
  modified: { letter: "M", variant: "modify", label: "Modified" },
  deleted: { letter: "D", variant: "delete", label: "Deleted" },
  renamed: { letter: "R", variant: "warning", label: "Renamed" },
  copied: { letter: "C", variant: "warning", label: "Copied" },
  typeChanged: { letter: "T", variant: "typechange", label: "Type changed" },
  untracked: { letter: "U", variant: "untracked", label: "Untracked" },
  unmerged: { letter: "!", variant: "error", label: "Conflict" },
};

export const ACTION_LABELS: Record<SourceControlActionKind, string> = {
  stage: "STAGE",
  unstage: "UNSTAGE",
  discard: "DISCARD",
  commit: "COMMIT",
  push: "PUSH",
  pull: "PULL",
};

export const READING_WORKING_TREE = "READING WORKING TREE";
export const NO_CHANGES = "NO CHANGES";
/** Nothing is listed, but staged paths above the configured root still travel with a commit */
export const NO_CHANGES_IN_FOLDER = "NO CHANGES IN THIS FOLDER";
export const STAGE_TO_COMMIT_HINT = "Stage changes to commit";
export const TRUNCATED_HINT = "Status truncated; refresh before committing";

export function reconcilingCopy(action: SourceControlActionKind): string {
  return `${ACTION_LABELS[action]} RESULT UNKNOWN — REFRESHING`;
}

export function actionErrorHeading(action: SourceControlActionKind, code: string): string {
  if (code === SOURCE_CONTROL_STATE_CHANGED_CODE) return "STATE CHANGED — REVIEW AGAIN";
  if (code === AGENT_DRAINING_CODE) return "AGENT SHUTTING DOWN";
  return `${ACTION_LABELS[action]} FAILED`;
}

/** A commit hook (e.g. lint-staged) re-staged reviewed-root paths; not an error, just informational */
export function hookChangedHeading(count: number): string {
  return `COMMIT HOOK CHANGED ${count} FILE(S)`;
}

export function hookChangedMessage(paths: string[]): string {
  return paths.join("\n");
}

/** What a discard does to one of the row's targets */
function discardEffectLabel(entry: SourceControlEntry, path: string): string {
  return entry.change === "untracked" && path === entry.path
    ? "deleted"
    : "restored from the index";
}

/** Every path the discard touches is named, because a row can own more than the file it shows */
export function discardConfirmMessage(entry: SourceControlEntry, targets: string[]): string {
  const lines = targets.map((path) => `${path} — ${discardEffectLabel(entry, path)}`);
  return `This cannot be undone:\n\n${lines.join("\n")}`;
}

interface StatusErrorCopy {
  heading: string;
  /** Whether the agent's message adds anything beyond the heading */
  showMessage: boolean;
}

export function statusErrorCopy(code: string | null): StatusErrorCopy {
  switch (code) {
    case "GIT_NOT_REPOSITORY":
      return { heading: "NOT A GIT REPOSITORY", showMessage: false };
    case "GIT_UNAVAILABLE":
      return { heading: "GIT NOT FOUND", showMessage: false };
    case "UNKNOWN_COMMAND":
      return { heading: "AGENT UPDATE REQUIRED", showMessage: false };
    case "GIT_UNSUPPORTED_VERSION":
      return { heading: "GIT VERSION UNSUPPORTED", showMessage: true };
    case "GIT_TIMEOUT":
      return { heading: "GIT TIMED OUT", showMessage: true };
    case "INVALID_REPO":
      return { heading: "REPO NOT CONFIGURED", showMessage: true };
    default:
      return { heading: "SOURCE CONTROL UNAVAILABLE", showMessage: true };
  }
}

export function shortSha(sha: string | null): string | null {
  return sha === null ? null : sha.slice(0, 7);
}

export function branchLabel(status: SourceControlStatus): string {
  if (status.detached) return `DETACHED ${shortSha(status.headSha) ?? ""}`.trim();
  if (status.branch !== null) return status.branch;
  return "(unborn)";
}

export function aheadBehindLabel(status: SourceControlStatus): string | null {
  if (status.upstream === null) return null;
  return `↑${status.ahead ?? 0} ↓${status.behind ?? 0}`;
}
