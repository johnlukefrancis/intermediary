// Path: app/src/components/source_control/source_control_copy.ts
// Description: Badge letters, branch labels, and empty-state/error copy for the Source Control column

import type {
  SourceControlActionKind,
  SourceControlStatus,
} from "../../shared/protocol.js";

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
export const STAGE_TO_COMMIT_HINT = "Stage changes to commit";
export const TRUNCATED_HINT = "Status truncated; refresh before committing";

export function reconcilingCopy(action: SourceControlActionKind): string {
  return `${ACTION_LABELS[action]} RESULT UNKNOWN — REFRESHING`;
}

export function actionErrorHeading(action: SourceControlActionKind): string {
  return `${ACTION_LABELS[action]} FAILED`;
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
