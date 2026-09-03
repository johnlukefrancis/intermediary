// Path: app/src/lib/source_control/tree_decorations.ts
// Description: Pure projection of a source-control status into per-file and rolled-up per-directory tree decorations

import type {
  SourceControlChange,
  SourceControlEntry,
  SourceControlStatus,
} from "../../shared/protocol.js";
import { CHANGE_BADGES } from "./change_badges.js";

export interface FileDecoration {
  change: SourceControlChange;
  letter: string;
  /** Suffix of the `.badge--*` class and the row's `data-change` value */
  variant: string;
  label: string;
}

export interface DirectoryDecoration {
  /** Distinct decorated paths anywhere beneath this directory */
  count: number;
  variant: string;
  label: string;
}

export interface TreeDecorations {
  files: ReadonlyMap<string, FileDecoration>;
  directories: ReadonlyMap<string, DirectoryDecoration>;
}

export const EMPTY_TREE_DECORATIONS: TreeDecorations = {
  files: new Map<string, FileDecoration>(),
  directories: new Map<string, DirectoryDecoration>(),
};

/** Worst-wins ranking for the directory roll-up; a closed union keeps the lookups defined. */
type Severity = 1 | 2 | 3 | 4;

const CHANGE_SEVERITY: Record<SourceControlChange, Severity> = {
  unmerged: 4,
  deleted: 3,
  modified: 2,
  renamed: 2,
  copied: 2,
  typeChanged: 2,
  added: 1,
  untracked: 1,
};

const SEVERITY_DECORATION: Record<Severity, { variant: string; label: string }> = {
  4: { variant: "error", label: "Conflicts beneath" },
  3: { variant: "delete", label: "Deleted beneath" },
  2: { variant: "modify", label: "Modified beneath" },
  1: { variant: "add", label: "New beneath" },
};

interface DirectoryRollUp {
  count: number;
  severity: Severity;
}

function fileDecoration(change: SourceControlChange): FileDecoration {
  const badge = CHANGE_BADGES[change];
  return { change, letter: badge.letter, variant: badge.variant, label: badge.label };
}

/** First writer wins, so callers pass areas in precedence order (conflicts, worktree, index). */
function addEntries(
  files: Map<string, FileDecoration>,
  entries: readonly SourceControlEntry[]
): void {
  for (const entry of entries) {
    if (files.has(entry.path)) continue;
    // Renamed and copied entries decorate the new path only; originalPath keeps its own row state.
    files.set(entry.path, fileDecoration(entry.change));
  }
}

function rollUpDirectories(
  files: ReadonlyMap<string, FileDecoration>
): Map<string, DirectoryDecoration> {
  const totals = new Map<string, DirectoryRollUp>();
  for (const [path, decoration] of files) {
    const severity = CHANGE_SEVERITY[decoration.change];
    const parts = path.split("/").filter(Boolean);
    for (let index = 1; index < parts.length; index += 1) {
      const ancestor = parts.slice(0, index).join("/");
      const total = totals.get(ancestor);
      if (total === undefined) {
        totals.set(ancestor, { count: 1, severity });
        continue;
      }
      total.count += 1;
      if (severity > total.severity) total.severity = severity;
    }
  }

  const directories = new Map<string, DirectoryDecoration>();
  for (const [path, total] of totals) {
    const { variant, label } = SEVERITY_DECORATION[total.severity];
    directories.set(path, { count: total.count, variant, label });
  }
  return directories;
}

/**
 * Deleted paths have no row on disk but still count toward, and color, their directories.
 * `omitted.*` paths are outside the configured root and never reach the tree.
 */
export function buildTreeDecorations(status: SourceControlStatus | null): TreeDecorations {
  if (status === null) return EMPTY_TREE_DECORATIONS;
  const files = new Map<string, FileDecoration>();
  addEntries(files, status.conflicts);
  addEntries(files, status.worktree);
  addEntries(files, status.index);
  return { files, directories: rollUpDirectories(files) };
}
