// Path: app/src/lib/source_control/status_groups.ts
// Description: Pure split of a status into the SOURCE rail's sections: merge conflicts, staged changes, changes

import type { SourceControlEntry, SourceControlStatus } from "../../shared/protocol.js";

export type StatusGroupKind = "conflicts" | "staged" | "changes";

export interface StatusGroup {
  kind: StatusGroupKind;
  entries: readonly SourceControlEntry[];
}

/** The rail's section order; CHANGES carries untracked paths, as in SOURCE. Empty groups are omitted. */
export function statusGroups(status: SourceControlStatus): StatusGroup[] {
  const groups: StatusGroup[] = [
    { kind: "conflicts", entries: status.conflicts },
    { kind: "staged", entries: status.index },
    { kind: "changes", entries: status.worktree },
  ];
  return groups.filter((group) => group.entries.length > 0);
}

/** Clean means a settled read listing nothing; reading, held and failed repositories are never clean */
export function isCleanStatus(status: SourceControlStatus | null): boolean {
  return status !== null && statusGroups(status).length === 0;
}
