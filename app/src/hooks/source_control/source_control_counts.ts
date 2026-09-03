// Path: app/src/hooks/source_control/source_control_counts.ts
// Description: SOURCE tab change count: distinct changed files, not area rows

import type { SourceControlStatus } from "../../shared/protocol.js";

/**
 * A file staged and edited again is one changed file, not two rows; staged paths above the
 * configured root are counted even though they have no row, because a commit carries them.
 */
export function countChangedPaths(status: SourceControlStatus): number {
  const paths = new Set<string>();
  for (const entry of [...status.index, ...status.worktree, ...status.conflicts]) {
    paths.add(entry.path);
  }
  return paths.size + status.omitted.stagedOutsideRoot;
}

/** Rows the three sections would render; zero means the body shows an empty state */
export function countVisibleRows(status: SourceControlStatus): number {
  return status.index.length + status.worktree.length + status.conflicts.length;
}
