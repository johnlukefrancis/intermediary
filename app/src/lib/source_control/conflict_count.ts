// Path: app/src/lib/source_control/conflict_count.ts
// Description: Conflicts that block a commit: listed unmerged paths plus unmerged paths above the configured root

import type { SourceControlStatus } from "../../shared/protocol.js";

/**
 * Git refuses a whole-index commit while any path is unmerged, including paths above a
 * subdirectory root that the projection counts in `omitted` but cannot list.
 */
export function totalConflictCount(status: SourceControlStatus): number {
  return status.conflicts.length + status.omitted.unmergedOutsideRoot;
}
